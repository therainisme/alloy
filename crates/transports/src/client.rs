use alloy_json_rpc::{Id, JsonRpcRequest, RpcParam, RpcReturn};
use serde_json::value::RawValue;
use tower::{Layer, ServiceBuilder};

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{BatchRequest, RpcCall, Transport};

#[derive(Debug)]
pub struct RpcClient<T> {
    pub(crate) transport: T,
    pub(crate) is_local: bool,
    pub(crate) id: AtomicU64,
}

impl<T> RpcClient<T> {
    pub fn new(t: T, is_local: bool) -> Self {
        Self {
            transport: t,
            is_local,
            id: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn is_local(&self) -> bool {
        self.is_local
    }

    #[inline]
    pub fn increment_id(&self) -> u64 {
        self.id.fetch_add(1, Ordering::Relaxed)
    }

    #[inline]
    pub fn next_id(&self) -> Id {
        Id::Number(self.increment_id())
    }
}

impl<T> RpcClient<T>
where
    T: Transport + Clone,
    T::Future: Send,
{
    #[inline]
    pub fn new_batch(&self) -> BatchRequest<T> {
        BatchRequest::new(self)
    }

    pub fn make_request<Params: RpcParam>(
        &self,
        method: &'static str,
        params: Params,
    ) -> JsonRpcRequest<Params> {
        JsonRpcRequest {
            method,
            params,
            id: self.next_id(),
        }
    }

    pub fn prepare<Params: RpcParam, Resp: RpcReturn>(
        &self,
        method: &'static str,
        params: Params,
    ) -> RpcCall<T, Params, Resp> {
        let request = self.make_request(method, params);
        RpcCall::new(request, self.transport.clone())
    }
}

pub struct ClientBuilder<L> {
    builder: ServiceBuilder<L>,
}

impl<L> ClientBuilder<L> {
    pub fn layer<M>(self, layer: M) -> ClientBuilder<tower::layer::util::Stack<M, L>> {
        ClientBuilder {
            builder: self.builder.layer(layer),
        }
    }

    pub fn transport<T>(self, transport: T) -> RpcClient<L::Service>
    where
        L: Layer<T>,
        T: Transport,
        L::Service: Transport + Clone,
        <L::Service as tower::Service<Box<RawValue>>>::Future: Send,
    {
        let is_local = transport.is_local();
        RpcClient::new(self.builder.service(transport), is_local)
    }
}

#[cfg(test)]
mod test {
    use crate::transports::http::Http;

    use super::RpcClient;

    #[test]
    fn basic_instantiation() {
        let h: RpcClient<Http<reqwest::Client>> = "http://localhost:8545".parse().unwrap();

        assert!(h.is_local());
    }
}

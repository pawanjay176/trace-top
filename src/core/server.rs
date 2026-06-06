use std::{error::Error, net::SocketAddr, sync::Arc};
use tonic::{Request, Response, Status as GrpcStatus, transport::Server};

use crate::core::{store::Store, types::NormalizedSpan};

use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
    trace_service_server::{TraceService, TraceServiceServer},
};

#[derive(Debug, Clone)]
pub struct OtlpTraceReceiver {
    store: Arc<Store>,
}

#[tonic::async_trait]
impl TraceService for OtlpTraceReceiver {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, GrpcStatus> {
        let request = request.into_inner();

        let mut spans: Vec<NormalizedSpan> = Vec::new();
        for resource_spans in request.resource_spans {
            for scope_spans in resource_spans.scope_spans {
                for span in scope_spans.spans {
                    spans.push(span.into());
                }
            }
        }
        self.store.insert_spans(spans);

        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

pub async fn serve(addr: &str, store: Arc<Store>) -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = addr.parse()?;
    let receiver = OtlpTraceReceiver { store };

    Server::builder()
        .add_service(TraceServiceServer::new(receiver))
        .serve(addr)
        .await?;

    Ok(())
}

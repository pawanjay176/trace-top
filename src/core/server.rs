use std::{error::Error, net::SocketAddr};
use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status as GrpcStatus, transport::Server};

use crate::core::{store::StoreReceiver, types::NormalizedSpan};

use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
    trace_service_server::{TraceService, TraceServiceServer},
};

#[derive(Debug, Clone)]
pub struct OtlpTraceReceiver {
    ingest_tx: mpsc::Sender<StoreReceiver>,
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
        self.ingest_tx
            .send(StoreReceiver::ReceivedSpans(spans))
            .await
            .map_err(|err| GrpcStatus::internal(format!("failed to ingest spans: {err}")))?;

        Ok(Response::new(ExportTraceServiceResponse {
            partial_success: None,
        }))
    }
}

pub async fn serve(
    addr: &str,
    ingest_tx: mpsc::Sender<StoreReceiver>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn Error>> {
    let addr: SocketAddr = addr.parse()?;
    let receiver = OtlpTraceReceiver { ingest_tx };

    Server::builder()
        .add_service(TraceServiceServer::new(receiver))
        .serve_with_shutdown(addr, async {
            let _ = shutdown_rx.await;
        })
        .await?;

    Ok(())
}

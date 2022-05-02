
use crate::webrtc::dtls_transport::RTCDtlsTransport;
use crate::webrtc::error::{Error, Result};
use crate::webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecParameters;
use crate::webrtc::rtp_transceiver::srtp_writer_future::SrtpWriterFuture;
use crate::webrtc::rtp_transceiver::{
    create_stream_info, PayloadType, RTCRtpEncodingParameters, RTCRtpSendParameters,
    RTCRtpTransceiver, SSRC,
};
use crate::webrtc::track::track_local::{
    InterceptorToTrackLocalWriter, TrackLocal, TrackLocalContext, TrackLocalWriter,
};

use ice::rand::generate_crypto_random_string;
use interceptor::stream_info::StreamInfo;
use interceptor::{Attributes, Interceptor, RTCPReader, RTPWriter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use tokio::sync::{mpsc, Mutex, Notify};

pub(crate) struct RTPSenderInternal {
    pub(crate) send_called_rx: Mutex<mpsc::Receiver<()>>,
    pub(crate) stop_called_rx: Arc<Notify>,
    pub(crate) stop_called_signal: Arc<AtomicBool>,
    pub(crate) rtcp_interceptor: Mutex<Option<Arc<dyn RTCPReader + Send + Sync>>>,
}

/// RTPSender allows an application to control how a given Track is encoded and transmitted to a remote peer
pub struct RTCRtpSender {
    pub(crate) track: Mutex<Option<Arc<dyn TrackLocal + Send + Sync>>>,

    pub(crate) srtp_stream: Arc<SrtpWriterFuture>,
    pub(crate) stream_info: Mutex<StreamInfo>,

    pub(crate) context: Mutex<TrackLocalContext>,

    pub(crate) transport: Arc<RTCDtlsTransport>,

    pub(crate) payload_type: PayloadType,
    pub(crate) ssrc: SSRC,
    receive_mtu: usize,

    /// a transceiver sender since we can just check the
    /// transceiver negotiation status
    pub(crate) negotiated: AtomicBool,

    pub(crate) interceptor: Arc<dyn Interceptor + Send + Sync>,

    pub(crate) id: String,

    rtp_transceiver: Mutex<Option<Weak<RTCRtpTransceiver>>>,

    send_called_tx: Mutex<Option<mpsc::Sender<()>>>,
    stop_called_tx: Arc<Notify>,
    stop_called_signal: Arc<AtomicBool>,

    internal: Arc<RTPSenderInternal>,
}

impl std::fmt::Debug for RTCRtpSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RTCRtpSender")
            .field("id", &self.id)
            .finish()
    }
}

impl RTCRtpSender {

    pub(crate) fn is_negotiated(&self) -> bool {
        self.negotiated.load(Ordering::SeqCst)
    }

    pub(crate) fn set_negotiated(&self) {
        self.negotiated.store(true, Ordering::SeqCst);
    }

    pub(crate) async fn set_rtp_transceiver(
        &self,
        rtp_transceiver: Option<Weak<RTCRtpTransceiver>>,
    ) {
        let mut tr = self.rtp_transceiver.lock().await;
        *tr = rtp_transceiver;
    }

    /// get_parameters describes the current configuration for the encoding and
    /// transmission of media on the sender's track.
    pub async fn get_parameters(&self) -> RTCRtpSendParameters {

        let send_parameters = {
            RTCRtpSendParameters {
                encodings: vec![RTCRtpEncodingParameters {
                    ssrc: self.ssrc,
                    payload_type: self.payload_type,
                    ..Default::default()
                }],
            }
        };

        send_parameters
    }

    /// track returns the RTCRtpTransceiver track, or nil
    pub async fn track(&self) -> Option<Arc<dyn TrackLocal + Send + Sync>> {
        let track = self.track.lock().await;
        track.clone()
    }

    /// replace_track replaces the track currently being used as the sender's source with a new TrackLocal.
    /// The new track must be of the same media kind (audio, video, etc) and switching the track should not
    /// require negotiation.
    pub async fn replace_track(
        &self,
        track: Option<Arc<dyn TrackLocal + Send + Sync>>,
    ) -> Result<()> {
        if let Some(t) = &track {
            let tr = self.rtp_transceiver.lock().await;
            if let Some(r) = &*tr {
                if let Some(r) = r.upgrade() {
                    if r.kind != t.kind() {
                        return Err(Error::ErrRTPSenderNewTrackHasIncorrectKind);
                    }
                } else {
                    //TODO: what about None arc?
                }
            } else {
                //TODO: what about None tr?
            }
        }

        if self.has_sent().await {
            let t = {
                let t = self.track.lock().await;
                t.clone()
            };
            if let Some(t) = t {
                let context = self.context.lock().await;
                t.unbind(&*context).await?;
            }
        }

        if !self.has_sent().await || track.is_none() {
            let mut t = self.track.lock().await;
            *t = track;
            return Ok(());
        }

        Ok(())
    }

    /// send Attempts to set the parameters controlling the sending of media.
    pub async fn send(&self, parameters: &RTCRtpSendParameters) -> Result<()> {
        if self.has_sent().await {
            return Err(Error::ErrRTPSenderSendAlreadyCalled);
        }

        let write_stream = Arc::new(InterceptorToTrackLocalWriter::new());
        let (context, stream_info) = {
            let track = self.track.lock().await;
            let context = TrackLocalContext {
                id: self.id.clone(),
            };

            let codec = if let Some(_) = &*track {
                return Err(Error::new("unsupported codec".to_string()));
            } else {
                RTCRtpCodecParameters::default()
            };
            let payload_type = codec.payload_type;
            let capability = codec.capability.clone();

            let stream_info = create_stream_info(
                self.id.clone(),
                parameters.encodings[0].ssrc,
                payload_type,
                capability,
                &vec![],
            );

            (context, stream_info)
        };

        let srtp_rtp_writer = Arc::clone(&self.srtp_stream) as Arc<dyn RTPWriter + Send + Sync>;
        let rtp_interceptor = self
            .interceptor
            .bind_local_stream(&stream_info, srtp_rtp_writer)
            .await;
        {
            let mut interceptor_rtp_writer = write_stream.interceptor_rtp_writer.lock().await;
            *interceptor_rtp_writer = Some(rtp_interceptor);
        }

        {
            let mut ctx = self.context.lock().await;
            *ctx = context;
        }
        {
            let mut si = self.stream_info.lock().await;
            *si = stream_info;
        }

        {
            let mut send_called_tx = self.send_called_tx.lock().await;
            send_called_tx.take();
        }

        Ok(())
    }

    /// stop irreversibly stops the RTPSender
    pub async fn stop(&self) -> Result<()> {
        if self.stop_called_signal.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.stop_called_signal.store(true, Ordering::SeqCst);
        self.stop_called_tx.notify_waiters();

        if !self.has_sent().await {
            return Ok(());
        }

        self.replace_track(None).await?;

        {
            let stream_info = self.stream_info.lock().await;
            self.interceptor.unbind_local_stream(&*stream_info).await;
        }

        self.srtp_stream.close().await
    }

    /// has_sent tells if data has been ever sent for this instance
    pub(crate) async fn has_sent(&self) -> bool {
        let send_called_tx = self.send_called_tx.lock().await;
        send_called_tx.is_none()
    }
}

#[macro_use]
extern crate lazy_static;

mod webrtc;

pub mod api {
    pub mod setting_engine {
        pub use crate::webrtc::api::setting_engine::SettingEngine;
    }
    pub use crate::webrtc::api::APIBuilder;
}
pub mod data_channel {
    pub mod data_channel_init {
        pub use crate::webrtc::data_channel::data_channel_init::RTCDataChannelInit;
    }
}
pub mod dtls_transport {
    pub mod dtls_role {
        pub use crate::webrtc::dtls_transport::dtls_role::DTLSRole;
    }
}
pub mod ice_transport {
    pub mod ice_candidate {
        pub use crate::webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
    }
}
pub mod peer_connection {
    pub mod configuration {
        pub use crate::webrtc::peer_connection::configuration::RTCConfiguration;
    }
    pub mod sdp {
        pub mod sdp_type {
            pub use crate::webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
        }
        pub mod session_description {
            pub use crate::webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
        }
    }
}
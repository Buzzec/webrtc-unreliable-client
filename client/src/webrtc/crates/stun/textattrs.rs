#[cfg(test)]
mod textattrs_test;

use crate::webrtc::stun::attributes::*;
use crate::webrtc::stun::checks::*;
use crate::webrtc::stun::error::*;
use crate::webrtc::stun::message::*;

use std::fmt;

const MAX_USERNAME_B: usize = 513;
const MAX_REALM_B: usize = 763;
const MAX_SOFTWARE_B: usize = 763;
const MAX_NONCE_B: usize = 763;

// Username represents USERNAME attribute.
//
// RFC 5389 Section 15.3
pub(crate) type Username = TextAttribute;

// Realm represents REALM attribute.
//
// RFC 5389 Section 15.7
pub(crate) type Realm = TextAttribute;

// Nonce represents NONCE attribute.
//
// RFC 5389 Section 15.8
pub(crate) type Nonce = TextAttribute;

// TextAttribute is helper for adding and getting text attributes.
#[derive(Clone, Default)]
pub(crate) struct TextAttribute {
    pub(crate) attr: AttrType,
    pub(crate) text: String,
}

impl fmt::Display for TextAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl Setter for TextAttribute {
    // add_to_as adds attribute with type t to m, checking maximum length. If max_len
    // is less than 0, no check is performed.
    fn add_to(&self, m: &mut Message) -> Result<()> {
        let text = self.text.as_bytes();
        let max_len = match self.attr {
            ATTR_USERNAME => MAX_USERNAME_B,
            ATTR_REALM => MAX_REALM_B,
            ATTR_SOFTWARE => MAX_SOFTWARE_B,
            ATTR_NONCE => MAX_NONCE_B,
            _ => return Err(Error::Other(format!("Unsupported AttrType {}", self.attr))),
        };

        check_overflow(self.attr, text.len(), max_len)?;
        m.add(self.attr, text);
        Ok(())
    }
}

impl Getter for TextAttribute {
    fn get_from(&mut self, m: &Message) -> Result<()> {
        let attr = self.attr;
        *self = TextAttribute::get_from_as(m, attr)?;
        Ok(())
    }
}

impl TextAttribute {
    pub(crate) fn new(attr: AttrType, text: String) -> Self {
        TextAttribute { attr, text }
    }

    // get_from_as gets t attribute from m and appends its value to reseted v.
    pub(crate) fn get_from_as(m: &Message, attr: AttrType) -> Result<Self> {
        match attr {
            ATTR_USERNAME => {}
            ATTR_REALM => {}
            ATTR_SOFTWARE => {}
            ATTR_NONCE => {}
            _ => return Err(Error::Other(format!("Unsupported AttrType {}", attr))),
        };

        let a = m.get(attr)?;
        let text = String::from_utf8(a)?;
        Ok(TextAttribute { attr, text })
    }
}

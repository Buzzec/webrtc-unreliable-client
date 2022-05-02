
use stun::attributes::ATTR_PRIORITY;
use stun::message::*;

/// Represents PRIORITY attribute.
#[derive(Default, PartialEq, Debug, Copy, Clone)]
pub struct PriorityAttr(pub u32);

const PRIORITY_SIZE: usize = 4; // 32 bit

impl Setter for PriorityAttr {
    // add_to adds PRIORITY attribute to message.
    fn add_to(&self, m: &mut Message) -> Result<(), stun::Error> {
        let mut v = vec![0_u8; PRIORITY_SIZE];
        v.copy_from_slice(&self.0.to_be_bytes());
        m.add(ATTR_PRIORITY, &v);
        Ok(())
    }
}
use crate::secure_channel::encryptor::KEY_RENEWAL_INTERVAL;
use crate::{IdentityError, Nonce};
use tracing_attributes::instrument;

/// fails compilation if [`KEY_RENEWAL_INTERVAL`] + 1 is bigger than [`BitmapType::BITS`].
///
/// the +1 is needed since the current nonce is also marked as received, taking an extra bit
/// even though we could check `current_nonce`, this compromise is for the sake of simplicity
const _: [(); (KEY_RENEWAL_INTERVAL + 1 > BitmapType::BITS as u64) as usize] = [];
type BitmapType = u64;

#[derive(Debug)]
pub(crate) struct NonceTracker {
    nonce_bitmap: BitmapType,
    current_nonce: Nonce,
}

impl NonceTracker {
    pub(crate) fn new() -> Self {
        Self {
            nonce_bitmap: 0,
            current_nonce: 0.into(),
        }
    }

    /// Mark a nonce as received, reject all invalid nonce values
    #[instrument(skip_all)]
    pub(crate) fn mark(&self, nonce: Nonce) -> ockam_core::Result<NonceTracker> {
        let new_tracker = if nonce > self.current_nonce {
            // normal case, we increase the nonce and move the window
            let relative_shift: u64 = nonce.value() - self.current_nonce.value();
            if relative_shift > KEY_RENEWAL_INTERVAL {
                return Err(IdentityError::InvalidNonce)?;
            }
            NonceTracker {
                nonce_bitmap: self.nonce_bitmap.overflowing_shl(relative_shift as u32).0 | 1,
                current_nonce: nonce,
            }
        } else {
            // first message or an out of order message
            let relative: u64 = self.current_nonce.value() - nonce.value();
            if relative > KEY_RENEWAL_INTERVAL {
                return Err(IdentityError::InvalidNonce)?;
            }

            #[allow(trivial_numeric_casts)]
            let bit = (1 as BitmapType).overflowing_shl(relative as u32).0;
            if self.nonce_bitmap & bit != 0 {
                // we already processed this nonce
                return Err(IdentityError::InvalidNonce)?;
            }
            NonceTracker {
                nonce_bitmap: self.nonce_bitmap | bit,
                current_nonce: self.current_nonce,
            }
        };

        Ok(new_tracker)
    }
}

#[test]
pub fn check_nonce_tracker() {
    let mut tracker = NonceTracker::new();
    tracker = tracker.mark(0.into()).unwrap();
    tracker = tracker.mark(1.into()).unwrap();
    tracker.mark(0.into()).unwrap_err();
    tracker.mark((KEY_RENEWAL_INTERVAL + 2).into()).unwrap_err();
    tracker = tracker.mark((KEY_RENEWAL_INTERVAL + 1).into()).unwrap();
    tracker.mark(1.into()).unwrap_err();
    tracker = tracker.mark((KEY_RENEWAL_INTERVAL + 2).into()).unwrap();
    tracker = tracker.mark((KEY_RENEWAL_INTERVAL + 3).into()).unwrap();
    tracker.mark((KEY_RENEWAL_INTERVAL + 1).into()).unwrap_err();
    tracker.mark((KEY_RENEWAL_INTERVAL + 2).into()).unwrap_err();
    tracker = tracker.mark((2 * KEY_RENEWAL_INTERVAL).into()).unwrap();
    tracker.mark((KEY_RENEWAL_INTERVAL - 1).into()).unwrap_err();
    tracker = tracker.mark((3 * KEY_RENEWAL_INTERVAL).into()).unwrap();
    tracker = tracker.mark((4 * KEY_RENEWAL_INTERVAL).into()).unwrap();
    for n in 3 * KEY_RENEWAL_INTERVAL + 1..4 * KEY_RENEWAL_INTERVAL {
        tracker = tracker.mark(n.into()).unwrap();
    }
    for n in 4 * KEY_RENEWAL_INTERVAL + 1..5 * KEY_RENEWAL_INTERVAL + 1 {
        tracker = tracker.mark(n.into()).unwrap();
    }
}

use arpy::{FnSubscription, MsgId};
use serde::{Deserialize, Serialize};
use serpent_automation_executor::run::ThreadRunState;

#[derive(MsgId, Serialize, Deserialize, Debug)]
pub struct ThreadSubscription;

impl FnSubscription for ThreadSubscription {
    type InitialReply = ();
    type Item = ThreadRunState;
    type Update = ();
}

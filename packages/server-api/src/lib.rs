use arpy::{FnSubscription, MsgId};
use serde::{Deserialize, Serialize};
use serpent_automation_executor::run::{CallStack, RunState};

#[derive(MsgId, Serialize, Deserialize, Debug)]
pub struct ThreadSubscription;

impl FnSubscription for ThreadSubscription {
    type InitialReply = ();
    type Item = (CallStack, RunState);
    type Update = CallStack;
}

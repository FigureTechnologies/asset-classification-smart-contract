use cosmwasm_std::CosmosMsg;
use provwasm_std::ProvenanceMsg;

pub trait MessageGatheringService {
    fn get_messages(&self) -> Vec<CosmosMsg<ProvenanceMsg>>;

    fn add_message(&self, message: CosmosMsg<ProvenanceMsg>);

    fn append_messages(&self, messages: &[CosmosMsg<ProvenanceMsg>]);

    fn clear_messages(&self);
}

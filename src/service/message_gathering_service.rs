use cosmwasm_std::CosmosMsg;
use provwasm_std::ProvenanceMsg;

/// Specifies a trait used for dynamically aggregating [CosmosMsg](cosmwasm_std::CosmosMsg) values
/// without requiring the owning struct to be mutable.
pub trait MessageGatheringService {
    /// Retrieves all messages that have been appended to the service.
    fn get_messages(&self) -> Vec<CosmosMsg<ProvenanceMsg>>;

    /// Moves an existing message into the service's collection of messages.
    fn add_message(&self, message: CosmosMsg<ProvenanceMsg>);

    /// Appends any number of existing messages by reference to the service.
    fn append_messages(&self, messages: &[CosmosMsg<ProvenanceMsg>]);

    /// Deletes all held messages from the service's internal values.
    fn clear_messages(&self);
}

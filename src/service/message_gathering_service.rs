use cosmwasm_std::CosmosMsg;

/// Specifies a trait used for dynamically aggregating [CosmosMsg](cosmwasm_std::CosmosMsg) values
/// without requiring the owning struct to be mutable.
pub trait MessageGatheringService {
    /// Retrieves all messages that have been appended to the service.
    fn get_messages(&self) -> Vec<CosmosMsg>;

    /// Moves an existing message into the service's collection of messages.
    fn add_message(&self, message: CosmosMsg);

    /// Appends any number of existing messages by reference to the service.
    fn append_messages(&self, messages: &[CosmosMsg]);

    /// Deletes all held messages from the service's internal values.
    fn clear_messages(&self);
}

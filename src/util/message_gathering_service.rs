use provwasm_std::ProvenanceMsg;

pub trait MessageGatheringService {
    // gets a vector of all messages generated in the order they were generated
    fn get_messages(&self) -> Vec<ProvenanceMsg>;

    // add a message to the end of the list of messages
    fn add_message(&mut self, message: ProvenanceMsg);
}

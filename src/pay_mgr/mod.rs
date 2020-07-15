use crate::module_bus::ModuleBus;
use std::collections::BTreeMap;


pub struct PayMgr {
    part_flows: BTreeMap<u64, ModuleBus>,
}

impl PayMgr {

    pub fn add_new_part_flow(&mut self, txid: u64, state_achine: ModuleBus) {
        self.part_flows.insert(txid, state_achine);
    }
}
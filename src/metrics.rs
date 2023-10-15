use subxt::config::substrate::H256;

#[derive(Default)]
pub struct Metrics {
    pub begin_send: u64,
    pub finalize_end: u64,
    pub first_tx_begin_block: Option<H256>,
    pub last_tx_finalize_block: Option<H256>,
    pub total_tx: u32,
}

impl Metrics {
    pub fn set_begin_block(&mut self, begin: H256) {
        if self.first_tx_begin_block == None {
            self.first_tx_begin_block = Some(begin)
        }
    }

    pub fn set_begin_timestamp(&mut self, begin_timestamp: u64){
        if self.begin_send == 0{
            self.begin_send = begin_timestamp
        }
    }

    pub fn set_end_timestamp(&mut self, end_timestamp: u64){
        self.finalize_end = end_timestamp
    }

    pub fn set_finalize_block(&mut self, finalize: H256) {
        self.last_tx_finalize_block = Some(finalize);
    }

    pub fn add_tx_number(&mut self, num: u32) {
        self.total_tx += num
    }
}

use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref GLOBAL_METRICS: Mutex<Metrics> = Mutex::new(Metrics {
        first_tx_begin_block: None,
        last_tx_finalize_block: None,
        total_tx: 0
    });
}

pub struct Metrics {
    pub first_tx_begin_block: Option<u32>,
    pub last_tx_finalize_block: Option<u32>,
    pub total_tx: u32,
}

impl Metrics {
    pub fn set_begin_block(&mut self, begin: u32) {
        if self.first_tx_begin_block == None {
            self.first_tx_begin_block = Some(begin)
        }
    }

    pub fn set_finalize_block(&mut self, finalize: u32) {
        if self.last_tx_finalize_block == None {
            self.last_tx_finalize_block = Some(finalize);
        }
    }

    pub fn add_tx_number(&mut self, num: u32) {
        self.total_tx += num
    }
}

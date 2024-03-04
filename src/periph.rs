use std::sync::mpsc;

pub trait mmap_peripheral {
	fn addr_base(&self) -> usize;
	fn addr_limit(&self) -> usize;
	fn read(&self, offset: usize) -> u8;
	fn write(&self, offset: usize, value: u8);
}

pub struct UART_tty;
impl mmap_peripheral for UART_tty {
    fn addr_base(&self) -> usize {
    	0x1000_0000
    }
	fn addr_limit(&self) -> usize {
		0x1000_0001
	}
    fn read(&self, offset: usize) -> u8 {
    	0
    }
    fn write(&self, offset: usize, value: u8) {
    	print!("{:}", value as char);
    }
}

pub struct UART_buffered {
	pub sink: mpsc::Sender<char>,
}
impl mmap_peripheral for UART_buffered {
    fn addr_base(&self) -> usize {
    	0x1000_0000
    }
	fn addr_limit(&self) -> usize {
		0x1000_0001
	}
    fn read(&self, offset: usize) -> u8 {
    	0
    }
    fn write(&self, offset: usize, value: u8) {
    	self.sink.send(value as char).unwrap();
    }
}


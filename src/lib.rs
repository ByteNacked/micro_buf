#![no_std]
#![allow(dead_code)]

/// Strategy:
///
/// empty
/// [ ][ ][ ][ ][ ][ ][ ]
///  ^
/// t h
/// t == h, size = 0
///
/// full
/// [*][*][*][*][*][*][*]
///  ^                 ^
/// t h                 
/// t == h, size = MAX
///
/// has elements
/// [ ][ ][*][ ][ ][ ][ ]
///        ^  ^
///        h  t

pub const RING_BUFFER_SZ : usize = 0x10000;

pub struct CircularBuffer {
    ring : [u8; RING_BUFFER_SZ], // 64 Kb
    head : usize,
    tail : usize,
    size : usize,
}

impl CircularBuffer {
    pub fn new() -> Self {
        CircularBuffer {
            ring : [0x0; RING_BUFFER_SZ],
            head : 0,
            tail : 0,
            size : 0,
        }
    }

    pub fn has_elements(&self) -> bool {
        self.size != 0
    }

    pub fn is_full(&self) -> bool {
        self.size == self.ring.len()
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn free_space(&self) -> usize {
        self.ring.len() - self.len()
    }

    fn empty(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.size = 0;
    }

    pub fn enqueue(&mut self, val : u8) {
        self.ring[self.tail] = val;
        self.tail = (self.tail + 1) % self.ring.len();
    }

    fn dequeue(&mut self) -> Option<u8> {
        if self.has_elements() {
            let val = self.ring[self.head];
            self.head = (self.head + 1) % self.ring.len();
            Some(val)
        }
        else {
            None
        }
    }

    /// Если буфер полон или не хватает места для записи новых элементов,
    /// удаляем старые чтобы вместить все добавляемые
    pub fn enqueue_slice(&mut self, val : &[u8]) -> bool {
        if val.len() > self.ring.len() {
            return false;
        }

        let (val_slice1, val_slice2) = {
            let space_on_rail = self.ring.len() - self.tail;

            let mid = if space_on_rail >= val.len() {
                val.len()
            }
            else {
                space_on_rail
            };

            val.split_at(mid)
        };

        let (ring_slice1, ring_slice2) = {
            let (sub_slice_1, sub_slice_2) = self.ring.split_at_mut(self.tail);

            let (ring_slice1, _) = sub_slice_2.split_at_mut(val_slice1.len());
            let (ring_slice2, _) = sub_slice_1.split_at_mut(val_slice2.len());

            (ring_slice1, ring_slice2)
        };

        ring_slice1.copy_from_slice(val_slice1);
        ring_slice2.copy_from_slice(val_slice2);

        self.size = self.size + val.len();
        self.tail = (self.tail + val.len()) % self.ring.len();

        if self.size > self.ring.len() {
            self.head = self.tail;
            self.size = self.ring.len();
        }

        true
    }

    /// Достаем элеметны из буфера, возвращаем количество элементов записанных в
    /// слайс. На данный момент переданный слайс или заполняется полностью
    /// (если достаточно элементов в буфере) или не заполняется вообще,
    /// возвращая длину ноль (если не достаточно элементов в буфере
    /// чтобы заполнить весь переданный слайс).
    pub fn dequeue_slice(&mut self, val : &mut [u8]) -> usize {
        // Если не достаточно элементов в буфере чтобы заполнить весь слайс, не достаем
        // элементы
        if val.len() > self.len() {
            return 0;
        }

        let (val_slice1, val_slice2) = {
            let space_on_rail = self.ring.len() - self.head;

            let mid = if space_on_rail >= val.len() {
                val.len()
            }
            else {
                space_on_rail
            };

            val.split_at_mut(mid)
        };

        let (ring_slice1, ring_slice2) = {
            let (sub_slice_1, sub_slice_2) = self.ring.split_at_mut(self.head);

            let (ring_slice1, _) = sub_slice_2.split_at(val_slice1.len());
            let (ring_slice2, _) = sub_slice_1.split_at(val_slice2.len());

            (ring_slice1, ring_slice2)
        };

        val_slice1.copy_from_slice(ring_slice1);
        val_slice2.copy_from_slice(ring_slice2);

        let elements_read = val_slice1.len() + val_slice2.len();
        self.size -= elements_read;
        self.head = (self.head + elements_read) % self.ring.len();

        elements_read
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buf_empty() {
        let buf = CircularBuffer::new();
        assert_eq!(buf.len(), 0)
    }

    #[test]
    fn test_buf_full() {
        let mut buf = CircularBuffer::new();
        let val = [0xAAu8; RING_BUFFER_SZ];
        let mut val2 = [0xBBu8; RING_BUFFER_SZ];

        assert!(buf.enqueue_slice(&val) == true);
        assert_eq!(buf.tail, 0);
        assert_eq!(buf.len(), RING_BUFFER_SZ);

        assert!(buf.dequeue_slice(&mut val2) == RING_BUFFER_SZ);
        assert_eq!(buf.len(), 0);

        assert_eq!(&val[..], &val2[..]);
    }

    #[test]
    fn test_buf_overwrite() {
        let mut buf = CircularBuffer::new();
        let val = [0xAAu8; RING_BUFFER_SZ / 2];
        let val2 = [0xBBu8; RING_BUFFER_SZ];
        let mut val3 = [0x0u8; RING_BUFFER_SZ];
        let val4 = [0xBBu8; RING_BUFFER_SZ];

        assert!(buf.enqueue_slice(&val) == true);
        assert_eq!(buf.len(), RING_BUFFER_SZ / 2);

        assert!(buf.enqueue_slice(&val2) == true);
        assert_eq!(buf.len(), RING_BUFFER_SZ);

        assert!(buf.dequeue_slice(&mut val3) == RING_BUFFER_SZ);
        assert_eq!(buf.len(), 0);


        assert_eq!(&val3[..], &val4[..]);
    }

    #[test]
    fn test_buf_overwrite2() {
        let mut buf = CircularBuffer::new();
        let val = {
            let mut val = [0x00u8; RING_BUFFER_SZ];
            for i in 0 .. RING_BUFFER_SZ / 4 {
                val[i * 4] = (i >> 0) as u8;
                val[i * 4 + 1] = (i >> 8) as u8;
                val[i * 4 + 2] = (i >> 16) as u8;
                val[i * 4 + 3] = (i >> 24) as u8;
            }
            val
        };
        // println!("{:?}", &val[..]);

        let val2 = {
            let mut val = [0x00u8; RING_BUFFER_SZ / 2];
            for i in RING_BUFFER_SZ / 4 .. RING_BUFFER_SZ / 4 + RING_BUFFER_SZ / 4 / 2 {
                let j = i - RING_BUFFER_SZ / 4;
                val[j * 4] = (i >> 0) as u8;
                val[j * 4 + 1] = (i >> 8) as u8;
                val[j * 4 + 2] = (i >> 16) as u8;
                val[j * 4 + 3] = (i >> 24) as u8;
            }
            val
        };
        // println!("{:?}", &val2[..]);

        let mut val3 = [0x0u8; RING_BUFFER_SZ];
        let val4 = {
            let mut val = [0x00u8; RING_BUFFER_SZ];
            for i in RING_BUFFER_SZ / 4 / 2 .. RING_BUFFER_SZ / 4 / 2 + RING_BUFFER_SZ / 4 {
                let j = i - RING_BUFFER_SZ / 4 / 2;
                val[j * 4] = (i >> 0) as u8;
                val[j * 4 + 1] = (i >> 8) as u8;
                val[j * 4 + 2] = (i >> 16) as u8;
                val[j * 4 + 3] = (i >> 24) as u8;
            }
            val
        };


        assert!(buf.enqueue_slice(&val) == true);
        assert_eq!(buf.len(), RING_BUFFER_SZ);

        // println!("head : {}", buf.head);
        // println!("tale : {}", buf.tail);

        assert!(buf.enqueue_slice(&val2) == true);
        assert_eq!(buf.len(), RING_BUFFER_SZ);

        // println!("head : {}", buf.head);
        // println!("tale : {}", buf.tail);
        assert!(buf.dequeue_slice(&mut val3) == RING_BUFFER_SZ);
        assert_eq!(buf.len(), 0);
        // println!("{:?}", &val3[..]);

        assert_eq!(&val3[..], &val4[..]);
    }

    #[test]
    fn test_buf_overwrite_linearity() {
        let mut buf = CircularBuffer::new();
        let val = {
            let mut val = [0x00u8; RING_BUFFER_SZ];
            for i in 0 .. RING_BUFFER_SZ / 4 {
                val[i * 4] = (i >> 0) as u8;
                val[i * 4 + 1] = (i >> 8) as u8;
                val[i * 4 + 2] = (i >> 16) as u8;
                val[i * 4 + 3] = (i >> 24) as u8;
            }
            val
        };

        let val2 = {
            let mut val = [0x00u8; RING_BUFFER_SZ / 2];
            for i in RING_BUFFER_SZ / 4 .. RING_BUFFER_SZ / 4 + RING_BUFFER_SZ / 4 / 2 {
                let j = i - RING_BUFFER_SZ / 4;
                val[j * 4] = (i >> 0) as u8;
                val[j * 4 + 1] = (i >> 8) as u8;
                val[j * 4 + 2] = (i >> 16) as u8;
                val[j * 4 + 3] = (i >> 24) as u8;
            }
            val
        };

        assert!(buf.enqueue_slice(&val) == true);
        assert_eq!(buf.len(), RING_BUFFER_SZ);

        assert!(buf.enqueue_slice(&val2) == true);
        assert_eq!(buf.len(), RING_BUFFER_SZ);

        let mut cnt_buf = [0xAAu8; 4];
        assert_eq!(buf.dequeue_slice(&mut cnt_buf), 4);
        // println!("{:X?}", &cnt_buf);
        let mut cnt : u32 = {
            let mut cnt : u32 = 0;
            cnt |= (cnt_buf[0] as u32) << 0;
            cnt |= (cnt_buf[1] as u32) << 8;
            cnt |= (cnt_buf[2] as u32) << 16;
            cnt |= (cnt_buf[3] as u32) << 24;
            cnt
        };
        // println!("{:X}", cnt);

        for _ in 1 .. RING_BUFFER_SZ / 4 {
            let mut cnt_buf = [0xAAu8; 4];
            assert_eq!(buf.dequeue_slice(&mut cnt_buf[0 .. 1]), 1);
            assert_eq!(buf.dequeue_slice(&mut cnt_buf[1 .. 2]), 1);
            assert_eq!(buf.dequeue_slice(&mut cnt_buf[2 .. 3]), 1);
            assert_eq!(buf.dequeue_slice(&mut cnt_buf[3 .. 4]), 1);
            // println!("{:?}", &cnt_buf);

            let cnt_new = {
                let mut cnt : u32 = 0;
                cnt |= (cnt_buf[0] as u32) << 0;
                cnt |= (cnt_buf[1] as u32) << 8;
                cnt |= (cnt_buf[2] as u32) << 16;
                cnt |= (cnt_buf[3] as u32) << 24;
                cnt
            };

            if cnt_new != cnt + 1 {
                // println!("cnt_new {:X}", cnt_new);
                panic!();
            }
            cnt = cnt_new;
            // println!("{:X}", cnt);
        }

        assert_eq!(buf.len(), 0);
    }
}

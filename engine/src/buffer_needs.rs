pub struct BufferNeeds {}

impl BufferNeeds {
    pub fn best_root(available: i32, size: i32) -> i32 {
        let available = available - 2;
        if available <= 1 {
            return 1;
        }
        let mut k = i32::MAX;
        let mut i = 1f32;
        while k > available {
            i += 1f32;
            k = (size as f32).powf(1.0 / i).ceil() as i32;
        }
        k
    }

    pub fn best_factor(availabe: i32, size: i32) -> i32 {
        let available = availabe - 2;
        if available <= 1 {
            return 1;
        }
        let mut k = size;
        let mut i: f32 = 1.0;
        while k > available {
            i += 1f32;
            k = (size as f32 / i).ceil() as i32;
        }
        k
    }
}

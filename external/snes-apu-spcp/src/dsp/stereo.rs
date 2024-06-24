#[derive(Copy, Clone, Debug, PartialEq)]
pub enum StereoChannel {
    Left,
    Right
}

impl StereoChannel {
    pub fn as_offset(&self) -> usize {
        match self {
            StereoChannel::Left => 0,
            StereoChannel::Right => 1
        }
    }
}

pub struct Stereo<T: Sized>(T, T);

impl<T: Sized> Stereo<T> {
    pub fn new(left: T, right: T) -> Self {
        Self(left, right)
    }

    pub fn left(&self) -> &T {
        &self.0
    }

    pub fn right(&self) -> &T {
        &self.1
    }

    pub fn set_left(&mut self, value: T) {
        self.0 = value;
    }

    pub fn set_right(&mut self, value: T) {
        self.1 = value;
    }

    pub fn set(&mut self, channel: StereoChannel, value: T) {
        match channel {
            StereoChannel::Left => self.0 = value,
            StereoChannel::Right => self.1 = value
        }
    }

    pub fn get(&self, channel: StereoChannel) -> &T {
        match channel {
            StereoChannel::Left => &self.0,
            StereoChannel::Right => &self.1
        }
    }

    pub fn get_mut(&mut self, channel: StereoChannel) -> &mut T {
        match channel {
            StereoChannel::Left => &mut self.0,
            StereoChannel::Right => &mut self.1
        }
    }

    pub fn into_inner_left(self) -> T {
        self.0
    }

    pub fn into_inner_right(self) -> T {
        self.1
    }

    pub fn into_inner(self, channel: StereoChannel) -> T {
        match channel {
            StereoChannel::Left => self.0,
            StereoChannel::Right => self.1
        }
    }
}

impl<T: Sized + Default> Default for Stereo<T> {
    fn default() -> Self {
        Self(T::default(), T::default())
    }
}

impl<T: Sized + Clone> Clone for Stereo<T> {
    fn clone(&self) -> Self {
        Self::new(self.0.clone(), self.1.clone())
    }
}

impl<T: Sized + Clone + Copy> Copy for Stereo<T> {}

use std::{
    alloc,
    borrow::{Borrow, BorrowMut},
    cell::Cell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use super::audio_buffer::AudioBuffer;

struct SharedAudioBufferBox {
    ref_count: Cell<usize>,
    value: AudioBuffer,
}

impl SharedAudioBufferBox {
    fn ref_count(&self) -> usize {
        self.ref_count.get()
    }

    fn inc_ref_count(&self) {
        let rc = self.ref_count();
        self.ref_count.set(rc + 1);
    }

    fn dec_ref_count(&self) {
        let rc = self.ref_count();
        self.ref_count.set(rc - 1);
    }
}

pub struct SharedAudioBuffer {
    ptr: NonNull<SharedAudioBufferBox>,
    phantom: PhantomData<SharedAudioBufferBox>,
}

impl SharedAudioBuffer {
    pub fn new(channels: usize, buffer_size: usize) -> Self {
        Self {
            ptr: Box::leak(Box::new(SharedAudioBufferBox {
                ref_count: Cell::new(1),
                value: AudioBuffer::new(channels, buffer_size),
            }))
            .into(),
            phantom: PhantomData,
        }
    }
}

impl AsMut<AudioBuffer> for SharedAudioBuffer {
    fn as_mut(&mut self) -> &mut AudioBuffer {
        self
    }
}

impl AsRef<AudioBuffer> for SharedAudioBuffer {
    fn as_ref(&self) -> &AudioBuffer {
        self
    }
}

impl Borrow<AudioBuffer> for SharedAudioBuffer {
    fn borrow(&self) -> &AudioBuffer {
        self
    }
}

impl BorrowMut<AudioBuffer> for SharedAudioBuffer {
    fn borrow_mut(&mut self) -> &mut AudioBuffer {
        self
    }
}

impl Clone for SharedAudioBuffer {
    fn clone(&self) -> Self {
        unsafe {
            self.ptr.as_ref().inc_ref_count();
        }
        Self {
            ptr: self.ptr.clone(),
            phantom: PhantomData,
        }
    }
}

impl fmt::Debug for SharedAudioBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

impl fmt::Display for SharedAudioBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

impl Deref for SharedAudioBuffer {
    type Target = AudioBuffer;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.ptr.as_ref().value }
    }
}

impl DerefMut for SharedAudioBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.ptr.as_mut().value }
    }
}

impl Drop for SharedAudioBuffer {
    fn drop(&mut self) {
        unsafe {
            self.ptr.as_ref().dec_ref_count();
            if self.ptr.as_ref().ref_count() == 0 {
                std::ptr::drop_in_place(self.ptr.as_ptr());
                alloc::dealloc(
                    self.ptr.as_ptr().cast(),
                    alloc::Layout::from_size_align_unchecked(
                        std::mem::size_of::<SharedAudioBufferBox>(),
                        std::mem::align_of::<SharedAudioBufferBox>(),
                    ),
                );
            }
        }
    }
}

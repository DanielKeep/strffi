use std::cell::RefCell;
use std::rc::Rc;

pub trait Utf8EncodeExt: Sized + Iterator<Item=char> {
    fn encode_utf8(self) -> Utf8EncodeIter<Self> {
        Utf8EncodeIter::new(self)
    }
}

impl<It> Utf8EncodeExt for It where It: Iterator<Item=char> {}

pub struct Utf8EncodeIter<It> where It: Iterator<Item=char> {
    iter: It,
    buf: [u8; 4],
    off: u8,
    len: u8,
}

impl<It> Utf8EncodeIter<It> where It: Iterator<Item=char> {
    pub fn new(iter: It) -> Self {
        Utf8EncodeIter {
            iter: iter,
            buf: [0; 4],
            off: 0,
            len: 0,
        }
    }
}

impl<It> Iterator for Utf8EncodeIter<It> where It: Iterator<Item=char> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len - self.off == 0 {
            // Buffer is empty; encode next code point.
            let cp = match self.iter.next() {
                Some(cp) => cp,
                None => return None,
            };
            let enc_str = cp.encode_utf8(&mut self.buf[..]);
            self.off = 0;
            self.len = enc_str.len() as u8;
        }

        let cu = self.buf[self.off as usize];
        self.off += 1;
        Some(cu)
    }
}

pub trait TrapErrExt: Sized + Iterator {
    type Trap;
    fn trap_err(self, trap: &mut Result<(), Self::Trap>) -> TrapErrIter<Self, Self::Trap>;
}

impl<It, T, E> TrapErrExt for It where It: Iterator<Item=Result<T, E>> {
    type Trap = E;

    fn trap_err(self, trap: &mut Result<(), Self::Trap>) -> TrapErrIter<Self, Self::Trap> {
        TrapErrIter {
            iter: Some(self),
            trap: trap,
        }
    }
}

pub struct TrapErrIter<'a, It, Trap: 'a> {
    iter: Option<It>,
    trap: &'a mut Result<(), Trap>,
}

impl<'a, It, T, E> Iterator for TrapErrIter<'a, It, E>
where
    It: Iterator<Item=Result<T, E>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let trapped = {
            let iter = match self.iter.as_mut() {
                Some(iter) => iter,
                None => return None,
            };

            match iter.next() {
                Some(Ok(e)) => return Some(e),
                Some(Err(err)) => Err(err),
                None => Ok(()),
            }
        };

        self.iter = None;
        *self.trap = trapped;
        None
    }
}

pub trait LiftErrExt: Sized + Iterator {
    type Trap;
    fn lift_err<Wrap, Over, U, F>(self, wrap: Wrap) -> LiftErrIter<Over, Self::Trap>
    where
        Wrap: FnOnce(LiftTrapErrIter<Self, Self::Trap>) -> Over,
        Over: Iterator<Item=Result<U, F>>,
        Self::Trap: Into<F>;
}

impl<It, T, E> LiftErrExt for It where It: Iterator<Item=Result<T, E>> {
    type Trap = E;

    fn lift_err<Wrap, Over, U, F>(self, wrap: Wrap) -> LiftErrIter<Over, Self::Trap>
    where
        Wrap: FnOnce(LiftTrapErrIter<Self, Self::Trap>) -> Over,
        Over: Iterator<Item=Result<U, F>>,
        Self::Trap: Into<F>,
    {
        let trap = Rc::new(RefCell::new(None));
        let middle = LiftTrapErrIter {
            iter: self,
            trap: trap.clone(),
        };
        let over = wrap(middle);
        LiftErrIter {
            iter: Some(over),
            trap: trap,
        }
    }
}

pub struct LiftErrIter<It, Err> {
    iter: Option<It>,
    trap: Rc<RefCell<Option<Err>>>,
}

impl<It, Err, LiftErr, T> Iterator for LiftErrIter<It, LiftErr>
where
    It: Iterator<Item=Result<T, Err>>,
    LiftErr: Into<Err>,
{
    type Item = Result<T, Err>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = match self.iter.as_mut() {
            Some(iter) => iter.next(),
            None => return None,
        };

        if let Some(err) = self.trap.borrow_mut().take() {
            self.iter = None;
            return Some(Err(err.into()));
        }

        next
    }
}

pub struct LiftTrapErrIter<It, Err> {
    iter: It,
    trap: Rc<RefCell<Option<Err>>>,
}

impl<It, Err, T> Iterator for LiftTrapErrIter<It, Err>
where
    It: Iterator<Item=Result<T, Err>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(Ok(v)) => Some(v),
            Some(Err(err)) => {
                *self.trap.borrow_mut() = Some(err);
                None
            },
            None => None,
        }
    }
}

pub trait Unsigned: Sized {
    type Unsigned;
    fn unsigned(self) -> Self::Unsigned;
}

impl Unsigned for u16 {
    type Unsigned = u16;
    fn unsigned(self) -> Self::Unsigned {
        self
    }
}

impl Unsigned for i32 {
    type Unsigned = u32;
    fn unsigned(self) -> Self::Unsigned {
        self as Self::Unsigned
    }
}

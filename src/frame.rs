use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Formatter,
    marker::PhantomData,
    rc::{Rc, Weak},
    fmt
};

use crate::{
    runtime::BuiltExpression,
    types::Value,
    parser::ast::Chunk
};

type WeakFrame = Weak<RefCell<__Frame>>;

/// Protected type for the frame.
/// 
/// The real type is `Frame`, which acts as a mutable pointer.
#[derive(Clone, Debug)]
struct __Frame {
    locals: HashMap<Rc<str>, Rc<Vec<Value>>>,
    macros: HashMap<Rc<str>, Rc<BuiltExpression>>,
    parent: Option<Frame>,
    children: Vec<WeakFrame>,
    pc: usize,
    chunk: Rc<Chunk>,
    security: usize,
}

#[derive(Clone, Debug)]
pub struct Frame {
    phantom_data: PhantomData<__Frame>,
    inner: Rc<RefCell<__Frame>>,
}

pub mod security {
    /// Marks the frame as unsecured, completely. The code can do anything.
    pub const ALL_UNSECURED: usize = WRITE | READ | ACCESS_PARENT;
    
    /// Marks the frame as writable.
    pub const WRITE: usize = 1;
    /// Marks the frame as readable.
    pub const READ: usize = 2;
    /// Specifies if the frame can access and modify the parent frame.
    /// 
    /// This is required for closure frames, for example.
    pub const ACCESS_PARENT: usize = 4;
    /// Marks the frame as sealed. A sealed frame cannot be a parent frame.
    /// 
    /// Note: [`ALL_UNSECURED`] doesn't enforce this because it's a security feature.
    pub const SEALED: usize = 8;
}

type FrameAccessResult<T> = Result<T, FrameAccessError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameAccessError {
    UnauthorizedAccess,
    InvalidOperation,
}

impl fmt::Display for FrameAccessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnauthorizedAccess => f.write_str("Unauthorized access to the frame"),
            Self::InvalidOperation => f.write_str("Invalid operation"),
        }
    }
}

use FrameAccessError::*;

impl Frame {
    fn assert_security(&self, security: usize) -> FrameAccessResult<()> {
        if self.inner.borrow().security & security == security {
            Ok(())
        } else {
            Err(UnauthorizedAccess)
        }
    }
    
    fn assert_not_sealed(&self) -> FrameAccessResult<()> {
        if self.inner.borrow().security & security::SEALED == 0 {
            Ok(())
        } else {
            Err(InvalidOperation)
        }
    }
    
    fn from_inner(frame: __Frame) -> Self {
        Self {
            phantom_data: PhantomData,
            inner: Rc::new(RefCell::new(frame)),
        }
    }
    
    pub fn new_unsecured(chunk: Rc<Chunk>) -> Self {
        Self::from_inner(__Frame {
            locals: HashMap::new(),
            macros: HashMap::new(),
            parent: None,
            children: Vec::new(),
            pc: 0,
            chunk,
            security: security::ALL_UNSECURED,
        })
    }
    
    pub fn new(chunk: Rc<Chunk>, security: FrameSecurity) -> Self {
        let frame = Frame::new_unsecured(chunk);
        frame.inner.borrow_mut().security = security.0;
        frame
    }

    pub fn new_child_unsecured(&self, chunk: Rc<Chunk>) -> FrameAccessResult<Self> {
        self.assert_not_sealed()?;

        let child = Self::from_inner(__Frame {
            locals: HashMap::new(),
            macros: HashMap::new(),
            parent: Some(self.clone()),
            children: Vec::new(),
            pc: 0,
            chunk,
            security: security::ALL_UNSECURED,
        });
        self.inner.borrow_mut().children.push(Rc::downgrade(&child.inner));
        Ok(child)
    }
    
    pub fn new_child(&self, chunk: Rc<Chunk>, security: FrameSecurity) -> FrameAccessResult<Self> {
        let frame = self.new_child_unsecured(chunk)?;
        frame.inner.borrow_mut().security = security.0;
        Ok(frame)
    }

    /// Pushes a new local variable into the frame.
    /// 
    /// Requires [`security::WRITE`] flag.
    pub fn push_local(&self, name: Rc<str>, value: Rc<Vec<Value>>) -> FrameAccessResult<()> {
        self.assert_security(security::WRITE)?;
        
        self.inner.borrow_mut().locals.insert(name, value);
        Ok(())
    }

    /// Pushes a new local macro into the frame.
    ///
    /// Requires [`security::WRITE`] flag.
    pub fn push_macro(&self, name: Rc<str>, expression: Rc<BuiltExpression>) -> FrameAccessResult<()> {
        self.assert_security(security::WRITE)?;
        
        self.inner.borrow_mut().macros.insert(name, expression);
        Ok(())
    }

    /// Drops a local variable from the frame.
    ///
    /// Requires [`security::WRITE`] flag.
    pub fn drop_local(&self, name: &str) -> FrameAccessResult<()> {
        self.assert_security(security::WRITE)?;
        
        self.inner.borrow_mut().locals.remove(name);
        Ok(())
    }

    /// Drops a local macro from the frame.
    ///
    /// Requires [`security::WRITE`] flag.
    pub fn drop_macro(&self, name: &str) -> FrameAccessResult<()> {
        self.assert_security(security::WRITE)?;
        
        self.inner.borrow_mut().macros.remove(name);
        Ok(())
    }

    /// Gets a local variable without recursive resolving.
    ///
    /// Requires [`security::READ`] flag.
    pub fn get_local(&self, name: &str) -> FrameAccessResult<Option<Rc<Vec<Value>>>> {
        self.assert_security(security::READ)?;
    
        Ok(self.inner.borrow().locals.get(name).cloned())
    }

    /// Gets a local macro without recursive resolving.
    /// 
    /// Requires [`security::READ`] flag.
    pub fn get_macro(&self, name: &str) -> FrameAccessResult<Option<Rc<BuiltExpression>>> {
        self.assert_security(security::READ)?;
    
        Ok(self.inner.borrow().macros.get(name).cloned())
    }

    /// Gets a local variable using recursion.
    /// 
    /// Requires [`security::READ`] and [`security::ACCESS_PARENT`] flags.
    pub fn resolve_local(&self, name: &str) -> FrameAccessResult<Option<Rc<Vec<Value>>>> {
        Ok('a: {
            let mut current = Some(self.clone());
            
            while let Some(frame) = current {
                frame.assert_security(security::READ)?;
                
                if let Some(value) = frame.get_local(name)? {
                    break 'a Some(value.clone());
                }

                frame.assert_security(security::ACCESS_PARENT)?;
                current = frame.inner.borrow().parent.clone(); 
            }
            
            None
        })
    }

    /// Gets a local macro using recursion.
    /// 
    /// Requires [`security::READ`] and [`security::ACCESS_PARENT`] flags.
    pub fn resolve_macro(&self, name: &str) -> FrameAccessResult<Option<Rc<BuiltExpression>>> {
        Ok('a: {
            let mut current = Some(self.clone());

            while let Some(frame) = current {
                frame.assert_security(security::READ)?;
                
                if let Some(expression) = frame.get_macro(name)? {
                    break 'a Some(expression.clone());
                }

                if frame.inner.borrow().parent.is_none() {
                    break;
                }
                
                frame.assert_security(security::ACCESS_PARENT)?;
                current = frame.inner.borrow().parent.clone();
            }

            None
        })
    }

    /// Updates a variable. Can update variable in parent frames.
    /// 
    /// Requires [`security::WRITE`], [`security::READ`] and [`security::ACCESS_PARENT`] flags.
    pub fn update_local(&self, name: &str, value: Rc<Vec<Value>>) -> FrameAccessResult<bool> {
        self.assert_security(security::WRITE | security::READ | security::ACCESS_PARENT)?;
        
        Ok('a: {
            let mut current = Some(self.clone());
            
            while let Some(frame) = current {
                frame.assert_security(security::READ | security::WRITE)?;
                
                let locals = &mut frame.inner.borrow_mut().locals;
                
                if locals.contains_key(name) {
                    locals.insert(name.into(), value.clone());
                    break 'a true;
                }
                
                if frame.inner.borrow().parent.is_none() {
                    break;
                }
                
                frame.assert_security(security::ACCESS_PARENT)?;
                current = frame.inner.borrow().parent.clone();
            }
            
            false
        })
    }

    pub fn update_macro(&self, name: &str, expression: Rc<BuiltExpression>) -> FrameAccessResult<bool> {
        self.assert_security(security::WRITE | security::READ | security::ACCESS_PARENT)?;
        
        Ok('a: {
            let mut current = Some(self.clone());
            
            while let Some(frame) = current {
                frame.assert_security(security::READ | security::WRITE)?;
                
                let macros = &mut frame.inner.borrow_mut().macros;
                
                if macros.contains_key(name) {
                    macros.insert(name.into(), expression.clone());
                    break 'a true;
                }
                
                if frame.inner.borrow().parent.is_none() {
                    break;
                }
                
                frame.assert_security(security::ACCESS_PARENT)?;
                current = frame.inner.borrow().parent.clone();
            }
            
            false
        })
    }
    
    /// Gets program counter.
    pub fn pc(&self) -> usize {
        self.inner.borrow().pc
    }
    
    /// Sets program counter.
    pub fn set_pc(&self, pc: usize) {
        self.inner.borrow_mut().pc = pc;
    }
    
    /// Gets the chunk that the frame belongs to.
    pub fn chunk(&self) -> Rc<Chunk> {
        self.inner.borrow().chunk.clone()
    }
}

pub struct FrameSecurity(usize);

impl FrameSecurity {
    pub fn new() -> Self {
        FrameSecurity(0)
    }
    
    pub fn with_write(mut self) -> Self {
        self.0 |= security::WRITE;
        self
    }
    
    pub fn with_read(mut self) -> Self {
        self.0 |= security::READ;
        self
    }
    
    pub fn with_access_parent(mut self) -> Self {
        self.0 |= security::ACCESS_PARENT;
        self
    }
    
    pub fn with_seal(mut self) -> Self {
        self.0 |= security::SEALED;
        self
    }
    
    pub fn with_all_unsecured(mut self) -> Self {
        self.0 |= security::ALL_UNSECURED;
        self
    }
}

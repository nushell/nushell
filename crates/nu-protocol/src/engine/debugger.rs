use std::any::Any;
use std::fmt::Debug;
use std::time::SystemTime;

use crate::ast::Call;
use crate::engine::{Command, EngineState, Stack};
use crate::{PipelineData, ShellError};

pub struct Wrapped2 {
    pub debugger: Box<dyn Debugger>
}

pub enum Wrapped {
    Direct(NoopDebugger),
    Indirect(Box<dyn Debugger>)
}
//
// impl Wrapped {
//     pub fn to_concrete<D: Debugger>(self) -> D {
//         match self {
//             Self::Direct(noop_debugger) => NoopDebugger,
//             Self::Indirect(debugger) => debugger.clone()
//         }
//     }
// }

// pub fn derive_debugger<D: Debugger>(debug_mode: impl DebugContext, debugger: &dyn Debugger) -> Wrapped<D> {
//     match debug_mode {
//         WithoutDebug => Wrapped::Direct(NoopDebugger),
//         WithDebug => Wrapped::Indirect(debugger.derive())
//     }
// }

/// Trait for static dispatching of eval_xxx() and debugger callback calls
pub trait DebugContext: Clone + Copy + Send {
    #[allow(unused_variables)]
    fn on_block_enter(&self, debugger: &mut dyn Debugger) {}
}

/// Marker struct signalizing that evaluation should use a Debugger
#[derive(Clone, Copy)]
pub struct WithDebug;

impl DebugContext for WithDebug {
    fn on_block_enter(&self, debugger: &mut dyn Debugger) {
        debugger.on_block_enter()
    }

    // fn derive_debugger<T: Debugger>(&self, debugger: &T) -> T {
    //     debugger.derive()
    // }
}

/// Marker struct signalizing that evaluation should NOT use a Debugger
#[derive(Clone, Copy)]
pub struct WithoutDebug;

impl DebugContext for WithoutDebug {}

// pub trait DebugData {
//     fn merge(&mut self, other: Box<dyn Any>) {}
// }
//
// pub struct BasicData {
//     pub timestamps: Vec<SystemTime>
// }
//
// impl DebugData for BasicData {
//     fn merge(&mut self, other: Box<dyn Any>) {
//         let data = other.downcast::<BasicData>().unwrap();
//         self.timestamps.extend(data.timestamps);
//     }
// }
//
// pub struct NoopData;

// impl DebugData for NoopData {}

/// Debugger trait that every debugger needs to implement.
///
/// By default, its callbacks are empty.
pub trait Debugger: Send {
    fn run_cmd(
        &mut self,
        decl: &Box<dyn Command>,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError>;

    fn on_block_enter(&mut self) {}

    // fn derive(&self) -> Wrapped { Wrapped::Direct(NoopDebugger)}
    fn derive(&self) -> Box<dyn Debugger> { Box::new(NoopDebugger)}

    // fn data(&self) -> &dyn DebugData { &NoopData}
    // fn into_data(self) -> Box<dyn DebugData> { Box::new(NoopData)}

    // fn as_any(&self) -> &dyn Any {
    //     &self
    // }

    fn as_any_mut(&mut self) -> &mut dyn Any;

    // fn merge(&mut self, other: Box<dyn Debugger>) {}
    fn merge(&mut self, other: &mut Box<dyn Debugger>) {}
}

/// Basic debugger showcasing the functionality
#[derive(Default)]
pub struct BasicDebugger {
    // pub data: BasicData
    pub timestamps: Vec<SystemTime>
}

impl Debugger for BasicDebugger {
    fn run_cmd(
        &mut self,
        decl: &Box<dyn Command>,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        decl.run_debug(engine_state, stack, call, input, self)
    }

    fn on_block_enter(&mut self) {
        self.timestamps.push(SystemTime::now());
        println!("Entered block with debugger!");
    }

    // fn derive(&self) -> Wrapped {
    //     Wrapped::Indirect(Box::new(BasicDebugger { timestamps: vec![]}))
    // }
    //
    // fn data(&self) -> &dyn DebugData {
    //     &self.data
    // }
    //
    // fn into_data(self) -> Box<dyn DebugData> {
    //     Box::new(self.data)
    // }
    //
    fn derive(&self) -> Box<dyn Debugger> {
        Box::new(BasicDebugger { timestamps: vec![]})
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn merge(&mut self, other: &mut Box<dyn Debugger>) {
        // let i = self.as_any_mut().downcast_mut::<BasicDebugger>().unwrap();
        let o = std::mem::take(other.as_any_mut().downcast_mut::<BasicDebugger>().unwrap());
        self.timestamps.extend(o.timestamps);
        // let other_basic: Box<dyn Any> = Box::new(*other);
        // let other2: Box<BasicDebugger> = other.as_any().downcast().unwrap();
        //
        // self.timestamps.extend(other2.timestamps);
        // self.merge(other.into_data())
    }
}

/// Noop debugger doing nothing, should not interfere with normal flow in any way.
pub struct NoopDebugger;

impl Debugger for NoopDebugger {
    fn run_cmd(
        &mut self,
        decl: &Box<dyn Command>,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        decl.run(engine_state, stack, call, input)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

use crate::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub transform: Transform,

    // Storing colors as raw 32-bit RGBA integers (e.g., 0xFF0000FF for red)
    // Using an Option allows us to know if the fill is "none"
    pub fill: Option<u32>,
    pub fill_rule: FillRule,
    pub stroke: Option<u32>,
    pub stroke_width: f32,
    pub opacity: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            transform: IDENTITY_TRANSFORM,
            fill: Some(0x000000FF), // Default SVG fill is solid black
            fill_rule: FillRule::NonZero,
            stroke: None,           // Default stroke is none
            stroke_width: 1.0,
            opacity: 1.0,
        }
    }
}

const MAX_DEPTH: usize = 64;

#[derive(Debug)]
pub struct StateStack {
    pub states: [State; MAX_DEPTH],
    pub top: usize,
}

impl StateStack {
    pub fn new() -> Self {
        Self {
            states: [State::default(); MAX_DEPTH],
            top: 0,
        }
    }

    /// Returns the current active state
    pub fn current(&self) -> &State {
        &self.states[self.top]
    }

    /// Returns a mutable reference to the current state to update attributes
    pub fn current_mut(&mut self) -> &mut State {
        &mut self.states[self.top]
    }

    /// Pushes a new state (entering a <g> tag)
    pub fn push(&mut self) {
        if self.top + 1 >= MAX_DEPTH {
            // In a production app, you might want to log a warning here,
            // but we simply clamp it to prevent a panic.
            return;
        }

        // Copy the current state exactly as it is to the next slot.
        // This automatically handles CSS/SVG inheritance (e.g., child gets parent's fill).
        self.states[self.top + 1] = self.states[self.top];
        self.top += 1;
    }

    /// Pops the current state (exiting a </g> tag)
    pub fn pop(&mut self) {
        if self.top > 0 {
            self.top -= 1;
        }
    }
}

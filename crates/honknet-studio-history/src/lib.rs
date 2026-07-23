use anyhow::Result;

pub trait EditorCommand: Send + Sync {
    fn apply(&mut self) -> Result<()>;
    fn revert(&mut self) -> Result<()>;
    fn description(&self) -> &str;
}

#[derive(Default)]
pub struct UndoRedoHistory {
    undo_stack: Vec<Box<dyn EditorCommand>>,
    redo_stack: Vec<Box<dyn EditorCommand>>,
    max_history: usize,
}

impl UndoRedoHistory {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: vec![],
            redo_stack: vec![],
            max_history: max_history.max(10),
        }
    }

    pub fn execute(&mut self, mut cmd: Box<dyn EditorCommand>) -> Result<()> {
        cmd.apply()?;
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool> {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.revert()?;
            self.redo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn redo(&mut self) -> Result<bool> {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.apply()?;
            self.undo_stack.push(cmd);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

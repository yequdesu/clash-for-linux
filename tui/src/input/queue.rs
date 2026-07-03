use std::collections::VecDeque;

use super::types::AppInput;

#[derive(Debug, Default)]
pub(crate) struct InputQueue {
    items: VecDeque<AppInput>,
}

impl InputQueue {
    pub(crate) fn push(&mut self, input: AppInput) {
        if matches!(input, AppInput::Resize { .. }) {
            self.items
                .retain(|item| !matches!(item, AppInput::Resize { .. }));
        }
        self.items.push_back(input);
    }

    pub(crate) fn pop(&mut self) -> Option<AppInput> {
        self.items.pop_front()
    }
}

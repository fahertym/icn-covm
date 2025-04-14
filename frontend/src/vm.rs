impl<S: Storage + Clone + Send + Sync + Debug + 'static> Clone for VM<S> {
    fn clone(&self) -> Self {
        VM {
            storage_backend: self.storage_backend.clone(),
            events: self.events.clone(),
            stack: self.stack.clone(),
            memory: self.memory.clone(),
            functions: self.functions.clone(),
            call_stack: self.call_stack.clone(),
            call_frames: self.call_frames.clone(),
            output: self.output.clone(),
            auth_context: self.auth_context.clone(),
            namespace: self.namespace.clone(),
            parameters: self.parameters.clone(),
            transaction_active: self.transaction_active,
        }
    }
} 
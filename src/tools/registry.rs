use crate::tools::{code_helper::CodeHelperTool, file_reader::FileReaderTool, homelab_api::HomelabApiTool, memory_recall::MemoryRecallTool, pdf_book_loader::PdfBookLoaderTool, shell_executor::ShellExecutorTool, web_search::WebSearchTool, Tool};

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool + Send + Sync>>,
}

pub struct ToolRegistryBuilder {
    tools: Vec<Box<dyn Tool + Send + Sync>>,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn add<T: Tool + Send + Sync + 'static>(mut self, tool: T) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub fn build(self) -> ToolRegistry {
        ToolRegistry { tools: self.tools }
    }
}

impl ToolRegistry {
    pub fn builder() -> ToolRegistryBuilder {
        ToolRegistryBuilder::new()
    }

    pub fn default() -> Self {
        Self::builder()
            .add(FileReaderTool)
            .add(PdfBookLoaderTool)
            .add(ShellExecutorTool)
            .add(WebSearchTool)
            .add(CodeHelperTool)
            .add(MemoryRecallTool)
            .add(HomelabApiTool)
            .build()
    }

    pub fn get(&self, tool_name: &str) -> Option<&(dyn Tool + Send + Sync)> {
        self.tools.iter().find_map(|tool| {
            if tool.name() == tool_name {
                Some(tool.as_ref())
            } else {
                None
            }
        })
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.name()).collect()
    }
}

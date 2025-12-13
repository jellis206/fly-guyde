use anyhow::{Context, Result};
use async_openai::{
    types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
            ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStrategy {
    Basic,        // Strategy 1: Schema-only (zero-shot)
    Intermediate, // Strategy 2: Schema + relationships + sample content
    Advanced,     // Strategy 3: Schema + relationships + content + few-shot examples
}

impl PromptStrategy {
    pub fn name(&self) -> &str {
        match self {
            PromptStrategy::Basic => "Basic (Zero-shot)",
            PromptStrategy::Intermediate => "Intermediate (Schema+Relations)",
            PromptStrategy::Advanced => "Advanced (Few-shot)",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            PromptStrategy::Basic => "Uses only the database schema (table and column names)",
            PromptStrategy::Intermediate => "Adds foreign key relationships and sample data",
            PromptStrategy::Advanced => "Includes demonstration examples of question-SQL pairs",
        }
    }
}

pub struct OpenAIClient {
    client: Client<async_openai::config::OpenAIConfig>,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self { client }
    }

    fn build_system_prompt(&self, strategy: PromptStrategy, schema_info: &str) -> String {
        let base_instructions = "You are an expert SQL query generator. Your task is to convert natural language questions into valid SQLite queries for a fly fishing product database.

IMPORTANT RULES:
1. Return ONLY valid SQLite SQL queries
2. Use proper SQLite syntax
3. Always use proper table and column names from the schema
4. Return SQL in a code block with ```sql
5. Do not include explanations unless asked
6. For product searches, remember to check the tags table for categorization
7. Use JOIN operations when querying across tables
8. Use LIKE with wildcards (%) for fuzzy text matching
9. Use aggregate functions (COUNT, AVG, MIN, MAX) when appropriate\n\n";

        match strategy {
            PromptStrategy::Basic => {
                format!("{}{}", base_instructions, schema_info)
            }
            PromptStrategy::Intermediate => {
                format!("{}{}\n\nUse the relationship information to properly JOIN tables when needed. Use sample data to understand the format and content of fields.", base_instructions, schema_info)
            }
            PromptStrategy::Advanced => {
                let examples = r#"
EXAMPLE DEMONSTRATIONS:

Question: "Show me all dry flies under $5"
SQL:
```sql
SELECT DISTINCT p.title, p.vendor, p.price_min
FROM products p
JOIN product_tags pt ON p.id = pt.product_id
JOIN tags t ON pt.tag_id = t.id
WHERE t.name = 'Dry Flies' AND p.price_min < 5.0
ORDER BY p.price_min;
```

Question: "What streamers does Umpqua make?"
SQL:
```sql
SELECT p.title, p.price_min, p.description
FROM products p
JOIN product_tags pt ON p.id = pt.product_id
JOIN tags t ON pt.tag_id = t.id
WHERE t.name = 'Streamers' AND p.vendor = 'Umpqua'
ORDER BY p.title;
```

Question: "Find the most expensive fly"
SQL:
```sql
SELECT title, vendor, price_max
FROM products
ORDER BY price_max DESC
LIMIT 1;
```

Question: "List all vendors and their product counts"
SQL:
```sql
SELECT vendor, COUNT(*) as product_count
FROM products
GROUP BY vendor
ORDER BY product_count DESC;
```
"#;
                format!("{}{}\n{}", base_instructions, schema_info, examples)
            }
        }
    }

    pub async fn generate_sql(
        &self,
        user_question: &str,
        schema_info: &str,
        strategy: PromptStrategy,
    ) -> Result<String> {
        let system_prompt = self.build_system_prompt(strategy, schema_info);

        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()?,
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(user_question)
                    .build()?,
            ),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .messages(messages)
            .temperature(0.0)
            .max_tokens(500u32)
            .build()?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .context("Failed to generate SQL from OpenAI")?;

        let sql = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .context("No response from OpenAI")?;

        Ok(self.extract_sql_from_response(sql))
    }

    fn extract_sql_from_response(&self, response: &str) -> String {
        if let Some(start) = response.find("```sql") {
            if let Some(end) = response[start..].find("```") {
                let sql_start = start + 6;
                let sql_end = start + end;
                return response[sql_start..sql_end].trim().to_string();
            }
        }

        if let Some(start) = response.find("SELECT") {
            return response[start..].trim().to_string();
        }

        response.trim().to_string()
    }

    pub async fn format_results_as_natural_language(
        &self,
        user_question: &str,
        sql: &str,
        result_count: usize,
    ) -> Result<String> {
        let prompt = format!(
            "The user asked: \"{}\"\n\nThe SQL query executed was:\n{}\n\nIt returned {} results.\n\nProvide a brief, natural language summary (1-2 sentences) of what was found. Be concise and helpful.",
            user_question, sql, result_count
        );

        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are a helpful assistant that summarizes database query results in natural language. Be concise and friendly.")
                    .build()?,
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()?,
            ),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .messages(messages)
            .temperature(0.7)
            .max_tokens(150u32)
            .build()?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .context("Failed to format results")?;

        let summary = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .context("No response from OpenAI")?;

        Ok(summary.trim().to_string())
    }
}

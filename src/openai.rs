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

QUERY DECISION LOGIC - Choose the right approach based on what the user is asking about:

1. FLY TYPE/CATEGORY queries (e.g., \"dry flies\", \"streamers\", \"nymphs\"):
   - USE: JOIN with product_tags and tags tables
   - Common tag names: 'Dry Flies', 'Streamers', 'Nymphs', 'Wet Flies', 'Emergers', 'Terrestrials', 'Saltwater Flies'
   - Pattern: WHERE t.name = 'Exact Tag Name'
   - Include DISTINCT to avoid duplicates

2. VENDOR queries (e.g., \"Umpqua flies\", \"flies from MFC\"):
   - USE: products.vendor column directly
   - Pattern: WHERE p.vendor = 'VendorName' OR WHERE p.vendor LIKE '%VendorName%'
   - NO need to join with tags

3. PRICE queries (e.g., \"cheapest flies\", \"flies under $5\"):
   - USE: products.price_min or products.price_max columns
   - Pattern: ORDER BY p.price_min ASC (for cheapest) or WHERE p.price_min < 5.0
   - Can combine with tags if asking for specific fly type

4. GENERAL product queries (e.g., \"show me some flies\", \"list products\"):
   - USE: products table only
   - Pattern: SELECT p.title, p.vendor, p.price_min FROM products p LIMIT N

5. COMBINED queries (e.g., \"cheapest dry flies from Umpqua\"):
   - Combine the appropriate patterns above
   - Join with tags if fly type is mentioned
   - Filter by vendor if vendor is mentioned
   - Order by price if price is relevant

COMMON PATTERNS:
- Vendor only: SELECT * FROM products WHERE vendor = 'Name'
- Fly type only: SELECT DISTINCT p.* FROM products p JOIN product_tags pt ON p.id = pt.product_id JOIN tags t ON pt.tag_id = t.id WHERE t.name = 'Type'
- Price sorting: ORDER BY price_min ASC/DESC
- Limiting results: LIMIT N (use reasonable limits like 10-50 if not specified)

\n\n";

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

Question: "List some Umpqua flies"
SQL:
```sql
SELECT title, vendor, price_min
FROM products
WHERE vendor = 'Umpqua'
ORDER BY title
LIMIT 20;
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
        // Try to find SQL in code blocks (```sql ... ```)
        if let Some(start) = response.find("```sql") {
            // Skip past the opening marker (```sql) and any whitespace/newlines
            let content_start = start + 6;
            let remaining = &response[content_start..];

            // Find the closing backticks (must be at least 3 backticks)
            if let Some(end) = remaining.find("```") {
                return remaining[..end].trim().to_string();
            }
        }

        // Try to find SQL in generic code blocks (``` ... ```)
        if let Some(start) = response.find("```") {
            let content_start = start + 3;
            let remaining = &response[content_start..];

            // Skip the language identifier if present
            let sql_start = if let Some(newline) = remaining.find('\n') {
                newline + 1
            } else {
                0
            };

            if let Some(end) = remaining[sql_start..].find("```") {
                return remaining[sql_start..sql_start + end].trim().to_string();
            }
        }

        // Fallback: look for SELECT statement
        if let Some(start) = response.find("SELECT") {
            // Try to extract just the SQL statement (stop at newlines followed by non-SQL text)
            let sql_portion = &response[start..];
            if let Some(end) = sql_portion.find("\n\n") {
                return sql_portion[..end].trim().to_string();
            }
            return sql_portion.trim().to_string();
        }

        // Last resort: return the whole response trimmed
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

    pub async fn get_fly_recommendations(&self, user_question: &str) -> Result<Vec<String>> {
        use crate::fuzzy::FuzzyMatcher;

        let prompt = format!(
            "Question: \"{}\"\n\nRecommend 3-5 specific fly patterns that would work well for this situation. Consider the target species, water conditions, and fishing scenario.\n\nOutput format: One fly name per line, nothing else.",
            user_question
        );

        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are an expert fly fishing guide. Recommend appropriate fly patterns based on the user's question. Your response must be ONLY fly pattern names, one per line. No greetings, no explanations, no extra text. Just list the fly names that are most relevant to their specific situation.")
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
            .max_tokens(200u32)
            .build()?;

        let response = self
            .client
            .chat()
            .create(request)
            .await
            .context("Failed to get fly recommendations from OpenAI")?;

        let content = response
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .context("No response from OpenAI")?;

        let recommendations: Vec<String> = content
            .lines()
            .filter_map(|line| FuzzyMatcher::clean_fly_name(line))
            .take(5)
            .collect();

        if recommendations.is_empty() {
            anyhow::bail!("No valid fly recommendations received from OpenAI");
        }

        Ok(recommendations)
    }
}

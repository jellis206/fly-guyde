# Fly-Guyde 🎣

A natural language interface for querying a fly fishing product database using AI-powered SQL generation.

## Project Description

Fly-Guyde is a Terminal User Interface (TUI) application that allows users to search through 2,112 fly fishing products from Fly Fish Food using natural language queries. The application uses OpenAI's GPT models (gpt-4o for SQL generation, gpt-4o-mini for summarization) to convert questions into SQL queries, executes them against a SQLite database, and presents results in an intuitive interface.

## Features

- 🤖 **AI-Powered Search**: Ask questions in plain English, get SQL-powered results
- 📊 **Three Prompting Strategies**: Compare different AI prompting approaches based on research
- 🖥️ **Clean TUI**: Interactive terminal interface with real-time feedback
- 🔍 **Full Product Database**: 2,117 products, 5,028 variants, 159 tags
- 📈 **Transparent Operations**: See the generated SQL and execution results

## Prompting Strategies

Based on the research paper ["How to Prompt LLMs for Text-to-SQL"](https://arxiv.org/abs/2305.11853), this project implements three distinct prompting strategies:

1. **Basic (Zero-shot)**: Uses only the database schema (table and column names)
2. **Intermediate**: Adds foreign key relationships and sample data to the prompt
3. **Advanced (Few-shot)**: Includes demonstration examples of question-SQL pairs

You can cycle through strategies using the **Tab** key to compare their effectiveness.

## Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Python 3**: For database setup (if regenerating from scratch)
- **OpenAI API Key**: Get one from [platform.openai.com](https://platform.openai.com/api-keys)

## Setup Instructions

### 1. Clone the Repository

```bash
git clone https://github.com/jellis206/fly-guyde.git
cd fly-guyde
```

### 2. Set Up OpenAI API Key

Create a `.env` file in the project root:

```bash
echo "OPENAI_API_KEY=sk-your-actual-key-here" > .env
```

Replace `sk-your-actual-key-here` with your actual OpenAI API key.

### 3. Create the Database

The database (`flies.db`) is not included in the repository. Generate it with a single command:

```bash
python3 seed_db.py
```

This fetches all ~2,100 products from the Fly Fish Food API, cleans the data, and builds `flies.db` with all tables and indexes. Takes ~1-2 minutes.

### 4. Build the Rust Application

```bash
cargo build --release
```

### 5. Run the Application

```bash
cargo run --release
```

## Usage

### Controls

**Press `?` at any time to see the full help screen!**

#### Main Controls
- **i / I**: Enter query mode (type your question)
- **Enter**: Submit your query
- **Esc**: Cancel query input
- **q / Q**: Quit the application
- **?**: Toggle help screen

#### Prompting Strategy Selection
- **1**: Basic (Zero-shot) strategy - Schema only
- **2**: Intermediate strategy - Schema + relationships + samples
- **3**: Advanced (Few-shot) strategy - Includes example queries

The current strategy is always displayed in the header.

### Example Queries

Try these questions:

- "Show me all dry flies under $5"
- "What streamers does Umpqua make?"
- "Find flies for euro nymphing"
- "Show the most expensive flies"
- "What flies are available from MFC?"
- "List all vendors and their product counts"
- "Find beadhead nymphs"
- "Show me stillwater flies"

### Strategy Comparison

Press **1**, **2**, or **3** to switch between the three prompting strategies and observe how they differ:

- **Basic (1)**: Fastest, but may struggle with complex queries
- **Intermediate (2)**: Better understanding of relationships and data types
- **Advanced (3)**: Most accurate, learns from examples, but uses more tokens

## Database Schema

### Tables

**products**
- id, handle, title, product_type, vendor, description
- price_min, price_max, available
- published_at, created_at, updated_at
- image_url

**variants**
- id, product_id (FK), title, sku
- price, available, inventory_quantity
- option1, option2, option3

**tags**
- id, name

**product_tags**
- product_id (FK), tag_id (FK)

### Views

- `product_details`: Denormalized view with aggregated tags

## Project Structure

```
fly-guyde/
├── src/
│   ├── main.rs         # Application entry point and main loop
│   ├── db.rs           # SQLite database operations
│   ├── openai.rs       # OpenAI API integration + prompting strategies
│   └── ui.rs           # TUI interface (ratatui)
├── seed_db.py          # Single script: scrape → clean → populate DB
├── schema.sql          # Database schema definition
├── Cargo.toml          # Rust dependencies
└── .env                # OpenAI API key (not in repo)
```

## Cost Estimates

Based on GPT-4o (SQL generation) and GPT-4o-mini (summarization) pricing:
- **Typical query**: ~$0.02-0.05
- **Testing session (10-20 queries)**: ~$0.20-1.00
- **Extended usage**: ~$0.50-5.00

The Basic strategy is cheapest (fewer tokens), while Advanced uses more tokens for examples.

## Troubleshooting

### "OPENAI_API_KEY not found"

Make sure you've created the `.env` file with your API key:
```bash
echo "OPENAI_API_KEY=sk-your-key-here" > .env
```

### "Failed to open database"

Make sure `flies.db` exists. If not, run:
```bash
python3 seed_db.py
```

### Build Errors

Make sure you have the latest stable Rust:
```bash
rustup update stable
```

### API Rate Limits

If you hit rate limits, wait a minute and try again. Consider adding a delay between queries during testing.

## Development

### Running in Debug Mode

```bash
cargo run
```

### Running Tests

```bash
cargo test
```

### Checking Code

```bash
cargo clippy
cargo fmt
```

## Data Source

Product data is sourced from [Fly Fish Food](https://www.flyfish-food.com/) via their product API. The database contains real fly fishing products from vendors like:
- Fulling Mill (817 products)
- Umpqua (608 products)
- Montana Fly Company (281 products)
- Solitude (206 products)
- And many more...

## Technologies Used

- **Rust**: Main application language
- **Ratatui**: Terminal UI framework
- **Crossterm**: Terminal manipulation
- **Tokio**: Async runtime
- **async-openai**: OpenAI API client
- **rusqlite**: SQLite database access
- **Python**: Data scraping and database setup

## Research Paper

This project implements prompting strategies from:

> Chang, S., & Fosler-Lussier, E. (2023). How to Prompt LLMs for Text-to-SQL: A Study in Zero-shot, Single-domain, and Cross-domain Settings. *NeurIPS 2023 Workshop on Table Representation Learning*.

[Read the paper](https://arxiv.org/abs/2305.11853)

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## Acknowledgments

- Fly Fish Food for product data
- OpenAI for GPT API
- Research by Chang & Fosler-Lussier on text-to-SQL prompting strategies

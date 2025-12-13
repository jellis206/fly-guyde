# Fly-Guyde Project Context

## Project Overview
**Name**: fly-guyde (pun on "fly guide")
**Purpose**: A natural language interface for querying a fly fishing product inventory from Fly Fish Food
**Type**: CS 452 class project - AI-powered SQL database interface

## Project Goals
1. Build a database of fly fishing products from Fly Fish Food
2. Create a TUI (Terminal User Interface) application in Rust
3. Implement natural language to SQL query interface using OpenAI GPT
4. Implement and compare all three prompting strategies from the research paper: https://arxiv.org/abs/2305.11853
5. Test with 6+ example queries and document results (both successful and failed)
6. Create schema visualization and documentation

## Technology Stack
- **Data Collection**: Python scripts
- **Database**: SQLite
- **TUI Application**: Rust with:
  - `ratatui` for terminal UI
  - `async-openai` crate for OpenAI API integration
  - `rusqlite` for SQLite database access
- **API**: OpenAI API (user has key in system environment, will also use .env file)
- **Version Control**: Git (with .env in .gitignore)

## Data Source
Fly Fish Food API: `https://services.mybcapps.com/bc-sf-filter/filter`

### API Details:
- **Total Products**: 2,117 fly fishing products
- **Pagination**: 28 products per page, 76 pages total
- **Base URL Parameters**:
  - shop: flyfishfood.myshopify.com
  - page: 1-76
  - limit: 28
  - sort: best-selling
  - collection_scope: 227410739365

### curl Example (Page 1):
```bash
curl 'https://services.mybcapps.com/bc-sf-filter/filter?t=1765440729967&_=pf&shop=flyfishfood.myshopify.com&page=1&limit=28&sort=best-selling&locale=en&event_type=collection&build_filter_tree=true&sid=25a94cfc-e11a-4bfa-8428-2fe2beb4ffc9&pg=collection_page&zero_options=true&product_available=false&variant_available=false&sort_first=available&urlScheme=2&collection_scope=227410739365' \
  -H 'accept: */*' \
  -H 'user-agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36'
```

## Project Structure

### Current Directory
- **Location**: `/Users/jayellis/projects/fly-guyde`
- **Note**: Directory was renamed from `fly-fishing` to `fly-guyde`

### Files Created

#### 1. `scraper.py`
Python script to fetch all products from Fly Fish Food API
- Fetches all 76 pages (2,117 products)
- Saves to `products.json`
- Includes error handling and progress tracking
- Rate limits requests (0.5s delay between pages)

**Status**: ✅ Completed and tested successfully

#### 2. `products.json`
Complete product data dump from API
- Contains 2,117 fly fishing products
- Each product includes: id, title, handle, type, vendor, description, price range, variants, tags, images, availability

**Status**: ✅ Generated successfully

#### 3. `schema.sql`
SQLite database schema with the following tables:

**products** table:
- id (PRIMARY KEY)
- handle (UNIQUE)
- title
- product_type
- vendor
- description
- price_min, price_max
- available (BOOLEAN)
- published_at, created_at, updated_at
- image_url

**variants** table:
- id (PRIMARY KEY)
- product_id (FOREIGN KEY)
- title
- sku
- price
- available
- inventory_quantity
- option1, option2, option3

**tags** table:
- id (PRIMARY KEY AUTOINCREMENT)
- name (UNIQUE)

**product_tags** table (junction table):
- product_id (FOREIGN KEY)
- tag_id (FOREIGN KEY)
- PRIMARY KEY (product_id, tag_id)

**Views**:
- `product_details`: Denormalized view with product info, variant count, and aggregated tags

**Indexes**: Created on commonly queried fields (product_type, vendor, price_min, available, SKU, etc.)

**Status**: ✅ Schema designed and saved

#### 4. `create_db.py`
Python script to create and populate the SQLite database
- Creates database from schema.sql
- Populates all tables from products.json
- Handles product-tag relationships
- Prints statistics and top tags/vendors

**Status**: ✅ Completed and tested successfully

#### 5. `flies.db`
SQLite database populated with all product data

**Database Statistics**:
- Products: 2,117
- Variants: 5,028
- Tags: 159 unique tags

**Top 10 Tags**:
1. Flies: 2,117 products
2. Dry Flies: 630 products
3. Nymphs: 572 products
4. Streamers: 538 products
5. Attractors: 496 products
6. Top Summer Flies: 490 products
7. Stillwater Flies: 462 products
8. Beadheads: 309 products
9. Euro Nymphing: 304 products
10. Mayflies: 256 products

**Vendors**:
- Fulling Mill: 817 products
- Umpqua: 608 products
- MFC: 281 products
- Solitude: 206 products
- Fulling Mill UK: 127 products
- Others: ~78 products

**Status**: ✅ Database created and populated successfully

### Other Files Present
- `sample_response.json`: First page API response (for testing)
- `peacock_bass_flies_0.json`: Previous test data
- `peacock_bass_flies_1.json`: Previous test data

## Next Steps (Not Yet Completed)

### 1. Initialize Rust Project
```bash
cd /Users/jayellis/projects/fly-guyde
cargo init --name fly-guyde .
```

### 2. Add Dependencies to Cargo.toml
Required crates:
- `ratatui` - TUI framework
- `crossterm` - Terminal manipulation
- `tokio` - Async runtime
- `async-openai` - OpenAI API client
- `rusqlite` - SQLite database
- `serde` and `serde_json` - JSON serialization
- `dotenv` - Load .env file
- `anyhow` or `thiserror` - Error handling

### 3. Create .env File
```bash
echo "OPENAI_API_KEY=your_key_here" > .env
echo ".env" >> .gitignore
```

### 4. Implement TUI Application
Key components:
- **Database module**: Connect to `flies.db`, execute SQL queries
- **OpenAI module**: Generate SQL from natural language using three prompting strategies
- **TUI module**: Render interface with ratatui
  - Input field for natural language queries
  - Display area for SQL query (for debugging/transparency)
  - Results table
  - Strategy selector (toggle between the 3 prompting strategies)

### 5. Implement Three Prompting Strategies
From paper https://arxiv.org/abs/2305.11853 (must read and implement):
1. **Strategy 1**: Basic prompting
2. **Strategy 2**: Intermediate prompting approach
3. **Strategy 3**: Advanced prompting approach

Need to experiment and document which works best.

### 6. Testing & Documentation
Create at least 6 test queries with results:
- At least one successful query example
- At least one failed query example
- Document SQL generated, results returned, and natural language response
- Compare results across all three prompting strategies

### 7. Schema Visualization
Create ERD (Entity Relationship Diagram) using one of:
- https://drawsql.app
- MySQL Workbench (reverse engineer)
- schemacrawler: `schemacrawler --server sqlite --database ./flies.db --command=schema --output-file=./schema.png --info-level=standard`
- Supabase schema visualizer

### 8. Final Deliverables for Class
- Working code (GitHub link)
- Database file (flies.db) or creation script
- Schema picture/ERD
- One-sentence project description
- Sample working query with SQL and response
- Sample failed query with SQL and response
- File with 6+ additional examples
- Documentation of which prompting strategies were tried and results
- Each team member (1-5 people) must:
  - Read the paper
  - Create schema picture
  - Contribute to database design
  - Execute program with their own OpenAI API key
  - Experiment with their own question

## Assignment Requirements Summary
- **Cost estimate**: ~$0.29 (from professor's example), should be under $5
- **Time estimate**: ~6 hours if coding from scratch
- **Key requirement**: Implement and compare prompting strategies from the paper
- **Creativity encouraged**: Fun and experimental project

## Current Status
✅ Data collection complete (scraper working)
✅ Database schema designed
✅ Database populated with 2,117 products
⏳ Rust TUI application - not started
⏳ OpenAI integration - not started
⏳ Prompting strategies - not started
⏳ Testing - not started
⏳ Documentation - not started

## Notes
- User prefers TUI over web interface
- User has OpenAI API key available in system environment
- Will use .env file for portability (anyone can pull repo and add their key)
- Pun name "fly-guyde" chosen by user
- Directory location had issues with tracking, but files are in `/Users/jayellis/projects/fly-guyde`

## Questions to Resolve Later
1. Should we implement a daily cron job to update the database? (User mentioned this - nice to have)
2. How to handle product updates? (Re-run scraper periodically?)
3. Which specific prompting strategies from the paper work best for this domain?

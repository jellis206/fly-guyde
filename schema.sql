-- Fly Fishing Product Database Schema
-- Stores fly fishing products from Fly Fish Food

-- Main products table
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY,
    handle TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    product_type TEXT,
    vendor TEXT,
    description TEXT,
    price_min REAL,
    price_max REAL,
    available BOOLEAN,
    published_at TEXT,
    created_at TEXT,
    updated_at TEXT,
    image_url TEXT
);

-- Product variants (different sizes, colors, etc.)
CREATE TABLE IF NOT EXISTS variants (
    id INTEGER PRIMARY KEY,
    product_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    sku TEXT,
    price REAL,
    available BOOLEAN,
    inventory_quantity INTEGER,
    option1 TEXT,
    option2 TEXT,
    option3 TEXT,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
);

-- Tags for categorization
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL
);

-- Many-to-many relationship between products and tags
CREATE TABLE IF NOT EXISTS product_tags (
    product_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (product_id, tag_id),
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_products_type ON products(product_type);
CREATE INDEX IF NOT EXISTS idx_products_vendor ON products(vendor);
CREATE INDEX IF NOT EXISTS idx_products_price_min ON products(price_min);
CREATE INDEX IF NOT EXISTS idx_products_available ON products(available);
CREATE INDEX IF NOT EXISTS idx_variants_product_id ON variants(product_id);
CREATE INDEX IF NOT EXISTS idx_variants_sku ON variants(sku);
CREATE INDEX IF NOT EXISTS idx_product_tags_product_id ON product_tags(product_id);
CREATE INDEX IF NOT EXISTS idx_product_tags_tag_id ON product_tags(tag_id);

-- View for easy querying with denormalized data
CREATE VIEW IF NOT EXISTS product_details AS
SELECT
    p.id,
    p.handle,
    p.title,
    p.product_type,
    p.vendor,
    p.description,
    p.price_min,
    p.price_max,
    p.available,
    p.image_url,
    COUNT(DISTINCT v.id) as variant_count,
    GROUP_CONCAT(DISTINCT t.name) as tags
FROM products p
LEFT JOIN variants v ON p.id = v.product_id
LEFT JOIN product_tags pt ON p.id = pt.product_id
LEFT JOIN tags t ON pt.tag_id = t.id
GROUP BY p.id;

#!/usr/bin/env python3
"""
Creates and populates the fly fishing database from scraped JSON data
"""
import json
import sqlite3
import sys
from pathlib import Path


def create_database(db_path='flies.db', schema_path='schema.sql'):
    """Create the database with schema"""
    print(f"Creating database: {db_path}")

    # Read schema
    with open(schema_path, 'r') as f:
        schema = f.read()

    # Create database and execute schema
    conn = sqlite3.connect(db_path)
    conn.executescript(schema)
    conn.commit()

    print("✓ Database schema created")
    return conn


def populate_database(conn, products_file='products.json'):
    """Populate database with products from JSON"""
    print(f"\nLoading products from {products_file}...")

    with open(products_file, 'r') as f:
        products = json.load(f)

    print(f"Processing {len(products)} products...")

    cursor = conn.cursor()

    # Track unique tags
    tags_map = {}

    for i, product in enumerate(products, 1):
        if i % 100 == 0:
            print(f"  Processed {i}/{len(products)} products...")

        # Extract first image URL if available
        image_url = None
        if 'images' in product and product['images']:
            # Images can be a dict or list
            if isinstance(product['images'], dict):
                image_url = next(iter(product['images'].values()), None)
            elif isinstance(product['images'], list):
                image_url = product['images'][0] if product['images'] else None

        # Insert product
        cursor.execute('''
            INSERT OR REPLACE INTO products (
                id, handle, title, product_type, vendor, description,
                price_min, price_max, available, published_at, created_at,
                updated_at, image_url
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            product.get('id'),
            product.get('handle'),
            product.get('title'),
            product.get('product_type'),
            product.get('vendor'),
            product.get('description'),
            product.get('price_min'),
            product.get('price_max'),
            product.get('available', False),
            product.get('published_at'),
            product.get('created_at'),
            product.get('updated_at'),
            image_url
        ))

        product_id = product.get('id')

        # Insert variants
        variants = product.get('variants', [])
        for variant in variants:
            cursor.execute('''
                INSERT OR REPLACE INTO variants (
                    id, product_id, title, sku, price, available,
                    inventory_quantity, option1, option2, option3
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ''', (
                variant.get('id'),
                product_id,
                variant.get('title'),
                variant.get('sku'),
                variant.get('price'),
                variant.get('available', False),
                variant.get('inventory_quantity', 0),
                variant.get('option1'),
                variant.get('option2'),
                variant.get('option3')
            ))

        # Handle tags
        tags = product.get('tags', [])
        for tag_name in tags:
            if tag_name not in tags_map:
                cursor.execute('INSERT OR IGNORE INTO tags (name) VALUES (?)', (tag_name,))
                cursor.execute('SELECT id FROM tags WHERE name = ?', (tag_name,))
                tag_id = cursor.fetchone()[0]
                tags_map[tag_name] = tag_id
            else:
                tag_id = tags_map[tag_name]

            # Link product to tag
            cursor.execute('''
                INSERT OR IGNORE INTO product_tags (product_id, tag_id)
                VALUES (?, ?)
            ''', (product_id, tag_id))

    conn.commit()
    print(f"✓ Successfully processed {len(products)} products")

    # Print statistics
    cursor.execute('SELECT COUNT(*) FROM products')
    product_count = cursor.fetchone()[0]

    cursor.execute('SELECT COUNT(*) FROM variants')
    variant_count = cursor.fetchone()[0]

    cursor.execute('SELECT COUNT(*) FROM tags')
    tag_count = cursor.fetchone()[0]

    print(f"\n=== Database Statistics ===")
    print(f"Products: {product_count}")
    print(f"Variants: {variant_count}")
    print(f"Tags: {tag_count}")

    # Show most common tags
    cursor.execute('''
        SELECT t.name, COUNT(*) as count
        FROM tags t
        JOIN product_tags pt ON t.id = pt.tag_id
        GROUP BY t.id
        ORDER BY count DESC
        LIMIT 10
    ''')

    print(f"\nTop 10 Tags:")
    for tag_name, count in cursor.fetchall():
        print(f"  {tag_name}: {count} products")

    # Show vendors
    cursor.execute('''
        SELECT vendor, COUNT(*) as count
        FROM products
        WHERE vendor IS NOT NULL
        GROUP BY vendor
        ORDER BY count DESC
    ''')

    print(f"\nVendors:")
    for vendor, count in cursor.fetchall():
        print(f"  {vendor}: {count} products")


def main():
    print("=== Fly Fishing Database Creator ===\n")

    # Check if products.json exists
    if not Path('products.json').exists():
        print("Error: products.json not found. Run scraper.py first.")
        return 1

    # Create database
    conn = create_database()

    # Populate database
    populate_database(conn)

    conn.close()
    print("\n✓ Database created successfully: flies.db")
    return 0


if __name__ == '__main__':
    sys.exit(main())

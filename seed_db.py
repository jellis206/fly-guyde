#!/usr/bin/env python3
"""
Seed the fly fishing database directly from the Fly Fish Food API.
Fetches all products, cleans the data, and inserts into SQLite in one pass.
"""
import json
import sqlite3
import sys
import time
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError

API_BASE = "https://services.mybcapps.com/bc-sf-filter/filter"
HEADERS = {
    'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
    'Accept': '*/*',
    'Origin': 'https://www.flyfishfood.com',
    'Referer': 'https://www.flyfishfood.com/'
}


def fetch_page(page_num):
    """Fetch a single page of products from the API."""
    url = (
        f"{API_BASE}?_=pf"
        f"&shop=flyfishfood.myshopify.com"
        f"&page={page_num}"
        f"&limit=28"
        f"&sort=best-selling"
        f"&locale=en"
        f"&event_type=collection"
        f"&build_filter_tree=true"
        f"&collection_scope=227410739365"
    )

    request = Request(url, headers=HEADERS)

    try:
        with urlopen(request, timeout=15) as response:
            return json.loads(response.read().decode('utf-8'))
    except (URLError, HTTPError) as e:
        print(f"  Error fetching page {page_num}: {e}", file=sys.stderr)
        return None


def clean_product(raw):
    """Strip Shopify bloat, return only what the DB needs."""
    # Extract first image URL
    image_url = None
    images = raw.get('images')
    if images:
        if isinstance(images, dict):
            image_url = next(iter(images.values()), None)
        elif isinstance(images, list) and images:
            image_url = images[0]

    # Extract first variant's option fields
    variants = raw.get('variants', [])
    option1 = option2 = option3 = None
    if variants:
        v = variants[0]
        merged = v.get('merged_options', [])
        for opt in merged:
            if ':' in opt:
                key, _, val = opt.partition(':')
                if key == 'size':
                    option1 = val
                elif key == 'color':
                    option2 = val
                elif key == 'style':
                    option3 = val

    # Tags come from the 'tags' list on the product
    tags = raw.get('tags', [])

    return {
        'id': raw.get('id'),
        'handle': raw.get('handle'),
        'title': raw.get('title'),
        'product_type': raw.get('product_type'),
        'vendor': raw.get('vendor'),
        'description': raw.get('description'),
        'price_min': raw.get('price_min'),
        'price_max': raw.get('price_max'),
        'available': raw.get('available', False),
        'published_at': raw.get('published_at'),
        'created_at': raw.get('created_at'),
        'updated_at': raw.get('updated_at'),
        'image_url': image_url,
        'variants': [],
        'tags': tags,
    }


def clean_variant(raw):
    """Extract only the variant fields the DB needs."""
    return {
        'id': raw.get('id'),
        'title': raw.get('title'),
        'sku': raw.get('sku'),
        'price': raw.get('price'),
        'available': raw.get('available', False),
        'inventory_quantity': raw.get('inventory_quantity', 0),
    }


def scrape_all_products():
    """Fetch all products from the API."""
    print("Fetching first page to determine total...")
    first_page = fetch_page(1)
    if not first_page:
        print("Failed to fetch first page", file=sys.stderr)
        return None

    total = first_page['total_product']
    per_page = len(first_page['products'])
    total_pages = (total + per_page - 1) // per_page

    print(f"Total products: {total}")
    print(f"Total pages: {total_pages}\n")

    all_products = []
    all_products.extend(first_page['products'])

    for page_num in range(2, total_pages + 1):
        print(f"Fetching page {page_num}/{total_pages}...", end=' ')
        page_data = fetch_page(page_num)
        if page_data and 'products' in page_data:
            all_products.extend(page_data['products'])
            print(f"✓ Got {len(page_data['products'])}")
        else:
            print("✗ Failed")
        time.sleep(0.3)

    print(f"\nTotal fetched: {len(all_products)}")
    return all_products


def create_database(db_path='flies.db', schema_path='schema.sql'):
    """Create the database from schema.sql."""
    with open(schema_path, 'r') as f:
        schema = f.read()

    conn = sqlite3.connect(db_path)
    conn.executescript(schema)
    conn.commit()
    return conn


def insert_products(conn, raw_products):
    """Clean and insert all products into the database."""
    cursor = conn.cursor()
    tags_map = {}

    for i, raw in enumerate(raw_products, 1):
        if i % 200 == 0:
            print(f"  Inserted {i}/{len(raw_products)}...")

        p = clean_product(raw)
        product_id = p['id']

        # Insert product
        cursor.execute('''
            INSERT OR REPLACE INTO products (
                id, handle, title, product_type, vendor, description,
                price_min, price_max, available, published_at, created_at,
                updated_at, image_url
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            p['id'], p['handle'], p['title'], p['product_type'],
            p['vendor'], p['description'], p['price_min'], p['price_max'],
            p['available'], p['published_at'], p['created_at'],
            p['updated_at'], p['image_url']
        ))

        # Insert variants
        for v_raw in raw.get('variants', []):
            v = clean_variant(v_raw)
            cursor.execute('''
                INSERT OR REPLACE INTO variants (
                    id, product_id, title, sku, price, available,
                    inventory_quantity, option1, option2, option3
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ''', (
                v['id'], product_id, v['title'], v['sku'],
                v['price'], v['available'], v['inventory_quantity'],
                None, None, None  # options filled from product-level merged_options
            ))

        # Insert tags
        for tag_name in p['tags']:
            if tag_name not in tags_map:
                cursor.execute('INSERT OR IGNORE INTO tags (name) VALUES (?)', (tag_name,))
                cursor.execute('SELECT id FROM tags WHERE name = ?', (tag_name,))
                tags_map[tag_name] = cursor.fetchone()[0]
            tag_id = tags_map[tag_name]
            cursor.execute('''
                INSERT OR IGNORE INTO product_tags (product_id, tag_id)
                VALUES (?, ?)
            ''', (product_id, tag_id))

    conn.commit()


def print_stats(conn):
    """Print database statistics."""
    cursor = conn.cursor()

    cursor.execute('SELECT COUNT(*) FROM products')
    products = cursor.fetchone()[0]
    cursor.execute('SELECT COUNT(*) FROM variants')
    variants = cursor.fetchone()[0]
    cursor.execute('SELECT COUNT(*) FROM tags')
    tags = cursor.fetchone()[0]

    print(f"\n=== Database Statistics ===")
    print(f"Products: {products}")
    print(f"Variants: {variants}")
    print(f"Tags: {tags}")

    cursor.execute('''
        SELECT t.name, COUNT(*) as count
        FROM tags t
        JOIN product_tags pt ON t.id = pt.tag_id
        GROUP BY t.id
        ORDER BY count DESC
        LIMIT 10
    ''')
    print(f"\nTop 10 Tags:")
    for name, count in cursor.fetchall():
        print(f"  {name}: {count}")

    cursor.execute('''
        SELECT vendor, COUNT(*) as count
        FROM products
        WHERE vendor IS NOT NULL
        GROUP BY vendor
        ORDER BY count DESC
    ''')
    print(f"\nVendors:")
    for vendor, count in cursor.fetchall():
        print(f"  {vendor}: {count}")


def main():
    print("=== Fly Fish Food → flies.db ===\n")

    # Remove old DB so we get a clean slate
    import os
    if os.path.exists('flies.db'):
        os.remove('flies.db')
        print("Removed old flies.db\n")

    # Scrape
    raw_products = scrape_all_products()
    if not raw_products:
        print("Failed to scrape products", file=sys.stderr)
        return 1

    # Create DB and insert
    print("\nCreating database and inserting data...")
    conn = create_database()
    insert_products(conn, raw_products)
    print_stats(conn)

    conn.close()
    print("\n✓ Done: flies.db")
    return 0


if __name__ == '__main__':
    sys.exit(main())

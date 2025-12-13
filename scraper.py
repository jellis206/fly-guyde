#!/usr/bin/env python3
"""
Fly Fish Food Product Scraper
Fetches all products from the Fly Fish Food API and saves to JSON
"""
import json
import time
import sys
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError


def fetch_page(page_num):
    """Fetch a single page of products"""
    url = (
        f"https://services.mybcapps.com/bc-sf-filter/filter"
        f"?_=pf"
        f"&shop=flyfishfood.myshopify.com"
        f"&page={page_num}"
        f"&limit=28"
        f"&sort=best-selling"
        f"&locale=en"
        f"&event_type=collection"
        f"&build_filter_tree=true"
        f"&collection_scope=227410739365"
    )

    headers = {
        'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
        'Accept': '*/*',
        'Origin': 'https://www.flyfishfood.com',
        'Referer': 'https://www.flyfishfood.com/'
    }

    request = Request(url, headers=headers)

    try:
        with urlopen(request, timeout=10) as response:
            return json.loads(response.read().decode('utf-8'))
    except (URLError, HTTPError) as e:
        print(f"Error fetching page {page_num}: {e}", file=sys.stderr)
        return None


def scrape_all_products():
    """Scrape all products from all pages"""
    print("Fetching first page to determine total...")

    first_page = fetch_page(1)
    if not first_page:
        print("Failed to fetch first page")
        return None

    total_products = first_page['total_product']
    products_per_page = len(first_page['products'])
    total_pages = (total_products + products_per_page - 1) // products_per_page

    print(f"Total products: {total_products}")
    print(f"Total pages: {total_pages}")
    print(f"Products per page: {products_per_page}\n")

    all_products = []
    all_products.extend(first_page['products'])

    # Fetch remaining pages
    for page_num in range(2, total_pages + 1):
        print(f"Fetching page {page_num}/{total_pages}...", end=' ')

        page_data = fetch_page(page_num)
        if page_data and 'products' in page_data:
            all_products.extend(page_data['products'])
            print(f"✓ Got {len(page_data['products'])} products")
        else:
            print("✗ Failed")

        # Be nice to the API
        time.sleep(0.5)

    print(f"\nTotal products fetched: {len(all_products)}")
    return all_products


def save_products(products, filename='products.json'):
    """Save products to JSON file"""
    with open(filename, 'w') as f:
        json.dump(products, f, indent=2)
    print(f"Saved to {filename}")


def main():
    print("=== Fly Fish Food Product Scraper ===\n")

    products = scrape_all_products()
    if products:
        save_products(products)

        # Print summary
        print("\n=== Summary ===")
        print(f"Total products: {len(products)}")

        # Count by type
        types = {}
        for p in products:
            ptype = p.get('product_type', 'Unknown')
            types[ptype] = types.get(ptype, 0) + 1

        print("\nProducts by type:")
        for ptype, count in sorted(types.items(), key=lambda x: x[1], reverse=True):
            print(f"  {ptype}: {count}")
    else:
        print("Failed to scrape products")
        return 1

    return 0


if __name__ == '__main__':
    sys.exit(main())

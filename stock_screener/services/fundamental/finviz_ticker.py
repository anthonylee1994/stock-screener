def extract_ticker_from_cell(col) -> str:
    """Extract the real ticker from a Finviz screener table cell.

    Finviz now renders a logo fallback letter plus the ticker link, so
    ``col.text`` becomes values like ``NNVDA`` instead of ``NVDA``. Prefer the
    explicit ticker attribute or the tab-link text when present.
    """
    attribute_ticker = col.get("data-boxover-ticker")
    if attribute_ticker:
        return str(attribute_ticker).strip()

    tab_link = col.find("a", class_="tab-link")
    if tab_link is not None:
        link_text = tab_link.get_text(strip=True)
        if link_text:
            return link_text

    return col.get_text(strip=True)

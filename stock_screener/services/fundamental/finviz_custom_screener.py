import pandas as pd
from finvizfinance.screener.custom import Custom
from finvizfinance.util import number_covert

from stock_screener.services.fundamental.finviz_ticker import extract_ticker_from_cell


class FinvizCustomScreener(Custom):
    """Finviz custom screener with corrected ticker cell parsing."""

    def _get_table(self, rows, df, num_col_index, table_header, limit=-1):
        rows = rows[1:]
        if limit != -1:
            rows = rows[0:limit]

        frame = []
        for row in rows:
            cols = row.find_all("td")[1:]
            info_dict = {}
            for i, col in enumerate(cols):
                header = table_header[i]
                if header == "Ticker":
                    info_dict[header] = extract_ticker_from_cell(col)
                elif i not in num_col_index:
                    info_dict[header] = col.text
                else:
                    info_dict[header] = number_covert(col.text)
            frame.append(info_dict)

        if len(df) == 0:
            return pd.DataFrame(frame)
        return pd.concat([df, pd.DataFrame(frame)], ignore_index=True)

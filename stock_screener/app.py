import logging
import os

from flask import Flask
from flask_cors import CORS

from stock_screener.controllers.screener_controller import screener_controller
from stock_screener.utils.stock_database import stock_database


logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s [%(name)s] %(message)s",
)


def create_app() -> Flask:
    app = Flask(__name__)
    CORS(app, resources={r"/*": {"origins": "*"}})
    screener_controller.register_routes(app)
    return app


app = create_app()
stock_database.initialize()


if __name__ == "__main__":
    app.run(host="0.0.0.0", port=int(os.environ.get("PORT", "3000")))

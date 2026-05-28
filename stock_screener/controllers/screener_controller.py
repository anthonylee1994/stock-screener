from flask import jsonify, request


class ScreenerController:
    def __init__(self, service=None):
        if service is None:
            from stock_screener.services.api.screener_service import screener_service

            service = screener_service
        self.service = service

    def register_routes(self, app) -> None:
        app.add_url_rule("/screener", view_func=self.screener, methods=["GET"])
        app.add_url_rule("/auth", view_func=self.auth, methods=["POST"])

    def get_request_payload(self) -> dict:
        payload = request.get_json(silent=True) or {}
        merged_payload = request.args.to_dict()
        merged_payload.update(payload)
        return merged_payload

    def screener(self):
        payload = self.get_request_payload()
        if not self.service.is_authorized(payload):
            return jsonify({"error": "Unauthorized"}), 401

        return jsonify(self.service.get_screener_response(payload))

    def auth(self):
        payload = self.get_request_payload()
        return jsonify({"authorized": self.service.is_authorized(payload)})


screener_controller = ScreenerController()

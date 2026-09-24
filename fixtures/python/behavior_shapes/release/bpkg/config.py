"""A settings singleton (Q04's shape): fields read at import, at construction and per call; one
never read, which a string-driven reader could still reach."""


class Settings:
    def __init__(self):
        self.host = "127.0.0.1"
        self.port = 8000
        self.debug = False
        self.log_level = "INFO"
        self.unused_option = None

    def get_setting(self, name):
        settings = self
        return getattr(settings, name)


class Plain:
    """No dynamic reader reaches it: a field never read is refuted under the model."""

    def __init__(self):
        self.used = 1
        self.never_read = 2

    def total(self):
        return self.used


settings = Settings()

LOG_LEVEL = settings.log_level

# Android home-screen widgets

`WidgetSync` publishes Monitor servers to Android's widget configuration screen.
The list and read-only credentials are passed to `MethodChans.publishWidgetServers`.
Android stores the list in shared preferences and keeps credentials in a
Keystore-wrapped file. Keep revocation and renewal tied to the server ID and
the Monitor endpoint when users edit or remove the server.

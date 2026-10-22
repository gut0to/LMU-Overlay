# Local Host Control

The overlay host exposes `\\.\pipe\HashOverlay.Host`. The wire format remains
compatible with existing Settings installations: one UTF-8 command and one
UTF-8 reply per connection. Frames are limited to 128 bytes. Surrounding
whitespace is accepted; empty, oversized, invalid UTF-8 and embedded-control
frames are rejected.

| Command | Successful reply |
| --- | --- |
| `status` | `running` or `stopped` |
| `reload` | `reloaded` |
| `show`, `hide`, `toggle_visibility` | `shown` or `hidden` |
| `visibility_status` | `shown` or `hidden` |
| `edit`, `edit_status` | `edit_on` or `edit_off` |
| `stop`, `shutdown` | `stopping` |

Unknown commands receive `invalid`. Reload failures use `error:<detail>`;
long messages are truncated at UTF-8 boundaries to fit the reply frame. Detailed
host logs remain the source for diagnosing a truncated message. Clients reject
partial writes and malformed replies instead of treating them as success.

Native hotkeys support F1–F12 with Ctrl/Control, Shift and Alt. Modifier order
does not affect identity; repeated modifiers and multiple function keys are
invalid. The shared parser is used for config validation and both native
registration paths.

The pipe adapter checks the documented INVALID_HANDLE_VALUE failure result for
[CreateNamedPipeW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createnamedpipew).
Transport IO is synchronous. Frame validation does not add an IO timeout; a
nonresponsive local peer remains a transport limitation requiring host restart.

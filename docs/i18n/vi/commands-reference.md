# Tham khảo lệnh clawclawclaw

Dựa trên CLI hiện tại (`clawclawclaw --help`).

Xác minh lần cuối: **2026-03-03**.

## Lệnh cấp cao nhất

| Lệnh | Mục đích |
|---|---|
| `onboard` | Khởi tạo workspace/config nhanh hoặc tương tác |
| `agent` | Chạy chat tương tác hoặc chế độ gửi tin nhắn đơn |
| `tui` | Chạy giao diện terminal toàn màn hình (cần feature `tui-ratatui`) |
| `gateway` | Khởi động gateway webhook và HTTP WhatsApp |
| `daemon` | Khởi động runtime có giám sát (gateway + channels + heartbeat/scheduler tùy chọn) |
| `service` | Quản lý vòng đời dịch vụ cấp hệ điều hành |
| `doctor` | Chạy chẩn đoán và kiểm tra trạng thái |
| `status` | Hiển thị cấu hình và tóm tắt hệ thống |
| `cron` | Quản lý tác vụ định kỳ |
| `models` | Làm mới danh mục model của provider |
| `providers` | Liệt kê ID provider, bí danh và provider đang dùng |
| `channel` | Quản lý kênh và kiểm tra sức khỏe kênh |
| `integrations` | Kiểm tra chi tiết tích hợp |
| `skills` | Liệt kê/cài đặt/gỡ bỏ skills |
| `migrate` | Nhập dữ liệu từ runtime khác (hiện hỗ trợ OpenClaw) |
| `config` | Kiểm tra, truy vấn và sửa đổi cấu hình runtime |
| `completions` | Tạo script tự hoàn thành cho shell ra stdout |
| `hardware` | Phát hiện và kiểm tra phần cứng USB |
| `peripheral` | Cấu hình và nạp firmware thiết bị ngoại vi |

## Nhóm lệnh

### `onboard`

- `clawclawclaw onboard`
- `clawclawclaw onboard --interactive`
- `clawclawclaw onboard --channels-only`
- `clawclawclaw onboard --api-key <KEY> --provider <ID> --memory <sqlite|lucid|markdown|none>`
- `clawclawclaw onboard --api-key <KEY> --provider <ID> --model <MODEL_ID> --memory <sqlite|lucid|markdown|none>`
- `clawclawclaw onboard --migrate-openclaw`
- `clawclawclaw onboard --migrate-openclaw --openclaw-source <PATH> --openclaw-config <PATH>`

### `agent`

- `clawclawclaw agent`
- `clawclawclaw agent -m "Hello"`
- `clawclawclaw agent --provider <ID> --model <MODEL> --temperature <0.0-2.0>`
- `clawclawclaw agent --peripheral <board:path>`

### `tui`

- `clawclawclaw tui`
- `clawclawclaw tui --provider <ID> --model <MODEL>`

Ghi chú:

- `tui` yêu cầu build với `--features tui-ratatui`.
- Khi thiếu feature, lệnh sẽ trả thông báo rebuild thân thiện.
- Phím tắt chính:
  - `Enter` gửi tin nhắn (chế độ nhập)
  - `Shift+Enter` xuống dòng
  - `Ctrl+C` hủy request đang chạy
  - nhấn `Ctrl+C` hai lần trong 300ms để thoát cưỡng bức
  - `q` hoặc `Ctrl+D` để thoát

### `gateway` / `daemon`

- `clawclawclaw gateway [--host <HOST>] [--port <PORT>] [--new-pairing]`
- `clawclawclaw daemon [--host <HOST>] [--port <PORT>]`

`--new-pairing` sẽ xóa toàn bộ token đã ghép đôi và tạo mã ghép đôi mới khi gateway khởi động.

### `service`

- `clawclawclaw service install`
- `clawclawclaw service start`
- `clawclawclaw service stop`
- `clawclawclaw service restart`
- `clawclawclaw service status`
- `clawclawclaw service uninstall`

### `cron`

- `clawclawclaw cron list`
- `clawclawclaw cron add <expr> [--tz <IANA_TZ>] <command>`
- `clawclawclaw cron add-at <rfc3339_timestamp> <command>`
- `clawclawclaw cron add-every <every_ms> <command>`
- `clawclawclaw cron once <delay> <command>`
- `clawclawclaw cron remove <id>`
- `clawclawclaw cron pause <id>`
- `clawclawclaw cron resume <id>`

### `models`

- `clawclawclaw models refresh`
- `clawclawclaw models refresh --provider <ID>`
- `clawclawclaw models refresh --force`

`models refresh` hiện hỗ trợ làm mới danh mục trực tiếp cho các provider: `openrouter`, `openai`, `anthropic`, `groq`, `mistral`, `deepseek`, `xai`, `together-ai`, `gemini`, `ollama`, `llamacpp`, `sglang`, `vllm`, `astrai`, `venice`, `fireworks`, `cohere`, `moonshot`, `stepfun`, `glm`, `zai`, `qwen`, `volcengine` (alias `doubao`/`ark`), `siliconflow` và `nvidia`.

### `channel`

- `clawclawclaw channel list`
- `clawclawclaw channel start`
- `clawclawclaw channel doctor`
- `clawclawclaw channel bind-telegram <IDENTITY>`
- `clawclawclaw channel add <type> <json>`
- `clawclawclaw channel remove <name>`

Lệnh trong chat khi runtime đang chạy (Telegram/Discord):

- `/models`
- `/models <provider>`
- `/model`
- `/model <model-id>`

Channel runtime cũng theo dõi `config.toml` và tự động áp dụng thay đổi cho:
- `default_provider`
- `default_model`
- `default_temperature`
- `api_key` / `api_url` (cho provider mặc định)
- `reliability.*` cài đặt retry của provider

`add/remove` hiện chuyển hướng về thiết lập có hướng dẫn / cấu hình thủ công (chưa hỗ trợ đầy đủ mutator khai báo).

### `integrations`

- `clawclawclaw integrations info <name>`

### `skills`

- `clawclawclaw skills list`
- `clawclawclaw skills install <source>`
- `clawclawclaw skills remove <name>`

`<source>` chấp nhận git remote (`https://...`, `http://...`, `ssh://...` và `git@host:owner/repo.git`) hoặc đường dẫn cục bộ.

Skill manifest (`SKILL.toml`) hỗ trợ `prompts` và `[[tools]]`; cả hai được đưa vào system prompt của agent khi chạy, giúp model có thể tuân theo hướng dẫn skill mà không cần đọc thủ công.

### `migrate`

- `clawclawclaw migrate openclaw [--source <path>] [--source-config <path>] [--dry-run]`

Gợi ý: trong hội thoại agent, bề mặt tool `openclaw_migration` cho phép preview hoặc áp dụng migration bằng tool-call có kiểm soát quyền.

### `config`

- `clawclawclaw config show`
- `clawclawclaw config get <key>`
- `clawclawclaw config set <key> <value>`
- `clawclawclaw config schema`

`config show` xuất toàn bộ cấu hình hiệu lực dưới dạng JSON với các trường nhạy cảm được ẩn thành `***REDACTED***`. Các ghi đè từ biến môi trường đã được áp dụng.

`config get <key>` truy vấn một giá trị theo đường dẫn phân tách bằng dấu chấm (ví dụ: `gateway.port`, `security.estop.enabled`). Giá trị đơn in trực tiếp; đối tượng và mảng in dạng JSON.

`config set <key> <value>` cập nhật giá trị cấu hình và lưu nguyên tử vào `config.toml`. Kiểu dữ liệu được suy luận tự động (`true`/`false` → bool, số nguyên, số thực, cú pháp JSON → đối tượng/mảng, còn lại → chuỗi). Sai kiểu sẽ bị từ chối trước khi ghi.

`config schema` xuất JSON Schema (draft 2020-12) cho toàn bộ hợp đồng `config.toml` ra stdout.

### `completions`

- `clawclawclaw completions bash`
- `clawclawclaw completions fish`
- `clawclawclaw completions zsh`
- `clawclawclaw completions powershell`
- `clawclawclaw completions elvish`

`completions` chỉ xuất ra stdout để script có thể được source trực tiếp mà không bị lẫn log/cảnh báo.

### `hardware`

- `clawclawclaw hardware discover`
- `clawclawclaw hardware introspect <path>`
- `clawclawclaw hardware info [--chip <chip_name>]`

### `peripheral`

- `clawclawclaw peripheral list`
- `clawclawclaw peripheral add <board> <path>`
- `clawclawclaw peripheral flash [--port <serial_port>]`
- `clawclawclaw peripheral setup-uno-q [--host <ip_or_host>]`
- `clawclawclaw peripheral flash-nucleo`

## Kiểm tra nhanh

Để xác minh nhanh tài liệu với binary hiện tại:

```bash
clawclawclaw --help
clawclawclaw <command> --help
```

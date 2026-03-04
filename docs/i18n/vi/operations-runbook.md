# Sổ tay Vận hành clawclawclaw

Tài liệu này dành cho các operator chịu trách nhiệm duy trì tính sẵn sàng, tình trạng bảo mật và xử lý sự cố.

Cập nhật lần cuối: **2026-02-18**.

## Phạm vi

Dùng tài liệu này cho các tác vụ vận hành day-2:

- khởi động và giám sát runtime
- kiểm tra sức khoẻ và chẩn đoán hệ thống
- triển khai an toàn và rollback
- phân loại và khôi phục sau sự cố

Nếu đây là lần cài đặt đầu tiên, hãy bắt đầu từ [one-click-bootstrap.md](one-click-bootstrap.md).

## Các chế độ Runtime

| Chế độ | Lệnh | Khi nào dùng |
|---|---|---|
| Foreground runtime | `clawclawclaw daemon` | gỡ lỗi cục bộ, phiên ngắn |
| Foreground gateway only | `clawclawclaw gateway` | kiểm thử webhook endpoint |
| User service | `clawclawclaw service install && clawclawclaw service start` | runtime được quản lý liên tục bởi operator |

## Checklist Cơ bản cho Operator

1. Xác thực cấu hình:

```bash
clawclawclaw status
```

1. Kiểm tra chẩn đoán:

```bash
clawclawclaw doctor
clawclawclaw channel doctor
```

1. Khởi động runtime:

```bash
clawclawclaw daemon
```

1. Để chạy như user session service liên tục:

```bash
clawclawclaw service install
clawclawclaw service start
clawclawclaw service status
```

## Tín hiệu Sức khoẻ và Trạng thái

| Tín hiệu | Lệnh / File | Kỳ vọng |
|---|---|---|
| Tính hợp lệ của config | `clawclawclaw doctor` | không có lỗi nghiêm trọng |
| Kết nối channel | `clawclawclaw channel doctor` | các channel đã cấu hình đều khoẻ mạnh |
| Tóm tắt runtime | `clawclawclaw status` | provider/model/channels như mong đợi |
| Heartbeat/trạng thái daemon | `~/.clawclawclaw/daemon_state.json` | file được cập nhật định kỳ |

## Log và Chẩn đoán

### macOS / Windows (log của service wrapper)

- `~/.clawclawclaw/logs/daemon.stdout.log`
- `~/.clawclawclaw/logs/daemon.stderr.log`

### Linux (systemd user service)

```bash
journalctl --user -u clawclawclaw.service -f
```

## Quy trình Phân loại Sự cố (Fast Path)

1. Chụp trạng thái hệ thống:

```bash
clawclawclaw status
clawclawclaw doctor
clawclawclaw channel doctor
```

1. Kiểm tra trạng thái service:

```bash
clawclawclaw service status
```

1. Nếu service không khoẻ, khởi động lại sạch:

```bash
clawclawclaw service stop
clawclawclaw service start
```

1. Nếu các channel vẫn thất bại, kiểm tra allowlist và thông tin xác thực trong `~/.clawclawclaw/config.toml`.

2. Nếu liên quan đến gateway, kiểm tra cài đặt bind/auth (`[gateway]`) và khả năng tiếp cận cục bộ.

## Quy trình Thay đổi An toàn

Trước khi áp dụng thay đổi cấu hình:

1. sao lưu `~/.clawclawclaw/config.toml`
2. chỉ áp dụng một thay đổi logic tại một thời điểm
3. chạy `clawclawclaw doctor`
4. khởi động lại daemon/service
5. xác minh bằng `status` + `channel doctor`

## Quy trình Rollback

Nếu một lần triển khai gây ra suy giảm hành vi:

1. khôi phục `config.toml` trước đó
2. khởi động lại runtime (`daemon` hoặc `service`)
3. xác nhận khôi phục qua `doctor` và kiểm tra sức khoẻ channel
4. ghi lại nguyên nhân gốc rễ và biện pháp khắc phục sự cố

## Tài liệu Liên quan

- [one-click-bootstrap.md](one-click-bootstrap.md)
- [troubleshooting.md](troubleshooting.md)
- [config-reference.md](config-reference.md)
- [commands-reference.md](commands-reference.md)

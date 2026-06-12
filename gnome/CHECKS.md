# Verification checklist

Service:

```bash
systemctl --user status tihulu-gnome-clipboard-daemon.service --no-pager
```

Text history:

```bash
printf 'verify text' | xclip -selection clipboard -in -target UTF8_STRING
sleep 2
~/.local/bin/tihulu-gnome-clipboard-helper state | grep 'verify text'
```

Encrypted history:

```bash
~/.local/bin/tihulu-gnome-clipboard-helper set encryptHistory true
printf 'verify encrypted' | xclip -selection clipboard -in -target UTF8_STRING
sleep 2
~/.local/bin/tihulu-gnome-clipboard-helper state | grep 'verify encrypted'
```

Image history:

Copy an image, wait a few seconds, then run:

```bash
~/.local/bin/tihulu-gnome-clipboard-helper state | grep -i image
```

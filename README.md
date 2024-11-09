# Timetracksync

A Telegram bot to upload my time sheets from my time tracking software
([Timemator](https://timemator.com/)) to the internal tracking software at my
employer (the Chair for Embedded Systems at RWTH Aachen University).

Usage:
- Obtain a Telegram bot token
- Run `timetracksync`: `timetracksync serve-telegram -a
  $AUTHORIZED_TELEGRAM_HANDLE -t $TELEGRAM_BOT_TOKEN $USERNAME $PASSWORD`, where
  `$USERNAME` is the username used for the tracking website (with
  `$PASSWORD` being the password). Now, you can send a CSV to your telegram bot
  (using `$TELEGRAM_BOT_TOKEN`). Your handle needs to be added as
  `$AUTHORIZED_TELEGRAM_HANDLE`. Unknown handles are rejected for security
  purposes.

## Security
This tool has only been hacked together in order to simplify the time tracking
process (and to learn Rust). It is only intended to be run on my personal
server. As such, the secret management approach is minimal. Use at your own
risk.

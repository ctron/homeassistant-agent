# Home Assistant Agent

[![crates.io](https://img.shields.io/crates/v/homeassistant-agent.svg)](https://crates.io/crates/homeassistant-agent)
[![docs.rs](https://docs.rs/homeassistant-agent/badge.svg)](https://docs.rs/homeassistant-agent)
[![GitHub release (latest SemVer)](https://img.shields.io/github/v/tag/ctron/homeassistant-agent?sort=semver)](https://github.com/ctron/homeassistant-agent/releases)
[![CI](https://github.com/ctron/homeassistant-agent/workflows/CI/badge.svg)](https://github.com/ctron/homeassistant-agent/actions?query=workflow%3A%22CI%22)

This crates helps in creating devices for Home Assistant MQTT integration.

> [!IMPORTANT]  
> This is an early experiment.

## ToDos

These are just some high-level notes:

* [ ] More type-safety: A switch needs a command topic, a binary sensor must send binary data
* [ ] Device topics must be more flexible, the command base is not a requirement
* [ ] Higher level abstractions for creating/managing sensors/devices

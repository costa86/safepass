# SafePass

## 1. Description

SafePass is a CLI password manager

## 2. Features
* Stores password on database
* Uses [Fernet (symmetric encryption)](https://github.com/fernet/spec/) to encrypt/decrypt passwords
* Sends passwords to clipboard (control + v)
* Uses a file as a `master key`


## 3. Instalation
### 3.1 Cargo

    cargo install safepass

### 3.2 Ready-to-use executable

|OS|Architecture| File*|
|--|--|--|
|Linux|x86_64|[safepass](https://github.com/costa86/safepass/blob/master/safepass)|

*Make sure you've granted executable permissions to it

    ./safepass

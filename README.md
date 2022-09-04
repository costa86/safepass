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

## 4. Important ⚠️
TLDR: Keep the security key file intact!

The security key is a file that works as a `master key` to encrypt/decrypt passwords in the database. If you tamper with it, you will lose access to all the passwords you have saved so far! It's located on the same path as the database, under the name `safepass.key`
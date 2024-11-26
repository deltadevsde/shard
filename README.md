# shard-sdk: minimal rollup framework

shard-sdk is a CLI tool designed to streamline the creation of simple based rollups on Celestia using a minimal template.

There is no bloat. There are no features. Because of the architectural simplicity of based rollups, all you need to implement is the verification and processing of transactions, as well as state management.

Examples coming soon.

## Installation
We use `just` as a task runner. After cloning the repo, run the following command to install the binary.

```bash
just install
```

## Usage

### Create a new project

```bash
shard init [rollup-name]
```

If no project name is provided, the default project name is “my-rollup”.

### Adding a new TransactionType

This will create a new transaction type with the specified fields and prepares both the transaction and state handling code automatically. Make sure you are in the rollup directory before you’re using `shard create-tx ...`

```bash
shard create-tx <tx-name> [field_name field_type]...
```

For example:

```bash
shard create-tx SendMessage msg String user String
```

After creating a new transaction type, you'll need to:

1. Update the `verify()` method in `src/tx.rs` to add your custom validation logic
2. Modify the `process()` method to implement the transaction logic

## Running the rollup

To start a local celestia network, run

```bash
just celestia-up
```

### Starting the node
After installing the binary for your rollup, you can run

```bash
my-rollup-name serve
```

### Creating a signer
If you have enabled signature verification, you will need to use signers. Generating signers to use with your rollup is easy:

```bash
my-rollup-name create-signer user1
```

### Submitting transcations

Let's say you used the `SendMessage` transaction type example above. To send a transaction, you can run:

```bash
my-rollup-name submit-tx send-message --key-name user1 --nonce 0 "Here is my message!" "Ryan"
```

You can omit the `--key-name` if signature verification is disabled, and `--nonce` if you haven't implemented nonce controls.

## Notes

Signature verification is disabled by default to allow for quick experimentation.

To enable it, change `SIGNATURE_VERIFICATION_ENABLED` in `your-rollup/src/tx.rs`  to `true` .

Nonce control is also not implemented by default. To prevent replay attacks, ensure processed transactions increment an account nonce. See example here [COMING SOON].

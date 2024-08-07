# Secret Contract Development Toolkit - SNIP52 (Private Push Notification) Interface

⚠️ This package is a sub-package of the `secret-toolkit` package. Please see its crate page for more context.

These functions are meant to help you easily create notification channels for private push notifications in secret contracts (see [SNIP-52 Private Push Notification](https://github.com/SolarRepublic/SNIPs/blob/feat/snip-52/SNIP-52.md)).

### Implementing a `NotificationData` struct

Each notification channel will have a specified data format, which is defined by creating a struct that implements the `NotificationData` trait, which has two methods: `to_cbor` and `channel_id`. The following example illustrates how you might implement this for a channel called `my_channel` and notification data containing two fields: `message` and `amount`.

```rust
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct MyNotificationData {
    pub message: String,
    pub amount: u128,
}

impl NotificationData for MyNotificationData {
    fn to_cbor(&self, _api: &dyn Api) -> StdResult<Vec<u8>> {
        let my_data = cbor::to_vec(&(
            self.message.as_bytes(),
            self.amount.to_be_bytes(),
        ))
        .map_err(|e| StdError::generic_err(format!("{:?}", e)))?;

        Ok(my_data)
    }

    fn channel_id(&self) -> &str {
        "my_channel"
    }
}
```

The `api` parameter for `to_cbor` is not used in this example, but is there for cases where you might want to convert an `Addr` to a `CanonicalAddr` before encoding using CBOR.

### Sending a TxHash notification

To send a notification to a recipient you then create a new `Notification` passing in the address of the recipient along with the notification data you want to send. The following creates a notification for the above `my_channel` and adds it to the contract `Response` as a plaintext attribute.

```rust
let note = Notification::new(
    recipient,
    MyNotificationData {
        "hello".to_string(),
        1000_u128,
    }
);

// ... other code

// add notification to response
Ok(Response::new()
    .set_data(to_binary(&ExecuteAnswer::MyMessage { status: Success } )?)
    .add_attribute_plaintext(
        note.id_plaintext(),
        note.data_plaintext(),
    )
)
```


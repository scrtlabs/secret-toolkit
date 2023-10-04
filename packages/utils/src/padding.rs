use cosmwasm_std::{Binary, Response};

/// Take a Vec<u8> and pad it up to a multiple of `block_size`, using spaces at the end.
pub fn space_pad(message: &mut Vec<u8>, block_size: usize) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}

/// Pad the data and logs in a `Result<Response, _>` to the block size, with spaces.
// Users don't need to care about it as the type `T` has a default, and will
// always be known in the context of the caller.
pub fn pad_handle_result<T, E>(
    response: Result<Response<T>, E>,
    block_size: usize,
) -> Result<Response<T>, E>
where
    T: Clone + std::fmt::Debug + PartialEq + schemars::JsonSchema,
{
    response.map(|mut response| {
        response.data = response.data.map(|mut data| {
            space_pad(&mut data.0, block_size);
            data
        });
        for attribute in &mut response.attributes {
            // do not pad plaintext attributes
            if attribute.encrypted {
                // Safety: These two are safe because we know the characters that
                // `space_pad` appends are valid UTF-8
                unsafe { space_pad(attribute.key.as_mut_vec(), block_size) };
                unsafe { space_pad(attribute.value.as_mut_vec(), block_size) };
            }
        }
        response
    })
}

/// Pad a `QueryResult` with spaces
pub fn pad_query_result<E>(response: Result<Binary, E>, block_size: usize) -> Result<Binary, E> {
    response.map(|mut response| {
        space_pad(&mut response.0, block_size);
        response
    })
}

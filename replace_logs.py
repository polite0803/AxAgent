import re

filepath = r'd:\OneManager\AxAgent\src-tauri\crates\gateway\src\handlers.rs'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update the macro to add $model_id parameter
old_macro_params = '($db:expr, $key:expr, $method:expr, $path:expr, $provider_id:expr, $status:expr, $elapsed:expr, $prompt:expr, $completion:expr, $error:expr)'
new_macro_params = '($db:expr, $key:expr, $method:expr, $path:expr, $model_id:expr, $provider_id:expr, $status:expr, $elapsed:expr, $prompt:expr, $completion:expr, $error:expr)'

content = content.replace(old_macro_params, new_macro_params, 1)

content = content.replace(
    '            $path,\n            None,\n            Some($provider_id),',
    '            $path,\n            $model_id,\n            Some($provider_id),',
    1
)

# 2. Replace standard calls (40 calls with &state.db, &gateway_key.id, &gateway_key.name, None, Some(&provider.id))
std_pattern = re.compile(
    r'( +)let _ = axagent_core::repo::gateway_request_log::record_request_log\(\n'
    r'\1    &state\.db,\n'
    r'\1    &gateway_key\.id,\n'
    r'\1    &gateway_key\.name,\n'
    r'\1    ("(?:GET|POST|DELETE|PATCH|PUT)"),\n'
    r'\1    (.+),\n'
    r'\1    None,\n'
    r'\1    Some\(&provider\.id\),\n'
    r'\1    (\d+),\n'
    r'\1    elapsed,\n'
    r'\1    (\d+),\n'
    r'\1    (\d+),\n'
    r'\1    (None),\n'
    r'\1\)\n'
    r'\1\.await\n'
    r'\1\.map_err\(\|e\| tracing::warn!\(%e, "Failed to record request log"\)\)\n'
    r'\1\.ok\(\);'
)

std_count = 0
def replace_std(m):
    global std_count
    std_count += 1
    indent = m.group(1)
    method = m.group(2)
    path = m.group(3)
    status = m.group(4)
    prompt = m.group(5)
    completion = m.group(6)
    error = m.group(7)
    return f'{indent}record_log!(&state.db, gateway_key, {method}, {path}, None, &provider.id, {status}, elapsed, {prompt}, {completion}, {error});'

content = std_pattern.sub(replace_std, content)
print(f"Standard calls replaced: {std_count}")

# 3. Replace handle_non_stream Ok case
ns_ok_pattern = re.compile(
    r'( +)let _ = axagent_core::repo::gateway_request_log::record_request_log\(\n'
    r'\1    &state\.db,\n'
    r'\1    &gateway_key\.id,\n'
    r'\1    &gateway_key\.name,\n'
    r'\1    "POST",\n'
    r'\1    "/v1/chat/completions",\n'
    r'\1    Some\(model_id\),\n'
    r'\1    Some\(provider_id\),\n'
    r'\1    200,\n'
    r'\1    elapsed,\n'
    r'\1    response\.usage\.prompt_tokens as i64,\n'
    r'\1    response\.usage\.completion_tokens as i64,\n'
    r'\1    None,\n'
    r'\1\)\n'
    r'\1\.await\n'
    r'\1\.map_err\(\|e\| tracing::warn!\(%e, "Failed to record request log"\)\)\n'
    r'\1\.ok\(\);'
)

ns_ok_count = 0
def replace_ns_ok(m):
    global ns_ok_count
    ns_ok_count += 1
    indent = m.group(1)
    return f'{indent}record_log!(&state.db, gateway_key, "POST", "/v1/chat/completions", Some(model_id), provider_id, 200, elapsed, response.usage.prompt_tokens as i64, response.usage.completion_tokens as i64, None);'

content = ns_ok_pattern.sub(replace_ns_ok, content)
print(f"Non-stream Ok calls replaced: {ns_ok_count}")

# 4. Replace handle_non_stream Err case
ns_err_pattern = re.compile(
    r'( +)let _ = axagent_core::repo::gateway_request_log::record_request_log\(\n'
    r'\1    &state\.db,\n'
    r'\1    &gateway_key\.id,\n'
    r'\1    &gateway_key\.name,\n'
    r'\1    "POST",\n'
    r'\1    "/v1/chat/completions",\n'
    r'\1    Some\(model_id\),\n'
    r'\1    Some\(provider_id\),\n'
    r'\1    502,\n'
    r'\1    elapsed,\n'
    r'\1    0,\n'
    r'\1    0,\n'
    r'\1    Some\(&e\.to_string\(\)\),\n'
    r'\1\)\n'
    r'\1\.await\n'
    r'\1\.map_err\(\|e\| tracing::warn!\(%e, "Failed to record request log"\)\)\n'
    r'\1\.ok\(\);'
)

ns_err_count = 0
def replace_ns_err(m):
    global ns_err_count
    ns_err_count += 1
    indent = m.group(1)
    return f'{indent}record_log!(&state.db, gateway_key, "POST", "/v1/chat/completions", Some(model_id), provider_id, 502, elapsed, 0, 0, Some(&e.to_string()));'

content = ns_err_pattern.sub(replace_ns_err, content)
print(f"Non-stream Err calls replaced: {ns_err_count}")

# 5. Handle stream case - restructure key cloning
content = content.replace(
    '        let key_id = gateway_key.id.clone();\n        let key_name = gateway_key.name.clone();',
    '        let key = gateway_key.clone();'
)

# 6. Replace handle_stream record_request_log call
stream_pattern = re.compile(
    r'( +)let _ = axagent_core::repo::gateway_request_log::record_request_log\(\n'
    r'\1    &db,\n'
    r'\1    &key_id,\n'
    r'\1    &key_name,\n'
    r'\1    "POST",\n'
    r'\1    "/v1/chat/completions",\n'
    r'\1    Some\(&mod_id\),\n'
    r'\1    Some\(&prov_id\),\n'
    r'\1    status_code,\n'
    r'\1    elapsed,\n'
    r'\1    total_prompt as i64,\n'
    r'\1    total_completion as i64,\n'
    r'\1    stream_error\.as_deref\(\),\n'
    r'\1\)\n'
    r'\1\.await\n'
    r'\1\.map_err\(\|e\| tracing::warn!\(%e, "Failed to record request log"\)\)\n'
    r'\1\.ok\(\);'
)

stream_count = 0
def replace_stream(m):
    global stream_count
    stream_count += 1
    indent = m.group(1)
    return f'{indent}record_log!(&db, key, "POST", "/v1/chat/completions", Some(&mod_id), &prov_id, status_code, elapsed, total_prompt as i64, total_completion as i64, stream_error.as_deref());'

content = stream_pattern.sub(replace_stream, content)
print(f"Stream calls replaced: {stream_count}")

# Verify
remaining = content.count('record_request_log')
print(f"Remaining record_request_log occurrences: {remaining} (expected: 1 in macro)")

total_replaced = std_count + ns_ok_count + ns_err_count + stream_count
print(f"\nTotal call replacements: {total_replaced}")

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("File written successfully!")

$file = 'd:\OneManager\AxAgent\src-tauri\crates\gateway\src\handlers.rs'
$content = [System.IO.File]::ReadAllText($file)

$search = @"
    let providers: Vec<ProviderConfig> =
        match axagent_core::repo::provider::list_providers(&state.db).await {
            Ok(p) => p
                .into_iter()
                .filter(|p| {
                    matches!(p.provider_type, ProviderType::OpenClaw | ProviderType::Hermes)
                })
                .collect(),
            Err(e) => {
                return error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string());
            },
        };

    let provider = match providers.first() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                "No Hermes/OpenClaw provider configured",
            );
        },
    };

    let provider_key =
        match axagent_core::repo::provider::get_active_key(&state.db, &provider.id).await {
            Ok(k) => k,
            Err(_) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("No active API key for provider '{}'", provider.name),
                );
            },
        };

    let api_key = match decrypt_key(&provider_key.key_encrypted, &state.master_key) {
        Ok(k) => k,
        Err(e) => {
            tracing::error!("Failed to decrypt provider key: {}", e);
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal key error");
        },
    };

    let global_settings = axagent_core::repo::settings::get_settings(&state.db)
        .await
        .unwrap_or_default();
    let resolved_proxy = ProviderProxyConfig::resolve(&provider.proxy_config, &global_settings);

    let ctx = ProviderRequestContext {
        api_key,
        key_id: provider_key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: resolved_proxy,
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry = axagent_providers::registry::ProviderRegistry::create_default();
    let adapter = match registry.get(provider_type_to_str(&provider.provider_type)) {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };
"@

$replace = @"
    let (provider, ctx, registry) = match resolve_hermes_provider_context(&state.db, &state.master_key).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    let adapter = match registry.get(provider_type_to_str(&provider.provider_type)) {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };
"@

$count = 0
$temp = $content
while ($temp.Contains($search)) {
    $idx = $temp.IndexOf($search)
    $temp = $temp.Remove($idx, $search.Length).Insert($idx, $replace)
    $count++
}

if ($count -gt 0) {
    [System.IO.File]::WriteAllText($file, $temp)
    Write-Output "Replaced $count occurrences"
} else {
    Write-Output 'No occurrences found - checking line endings'
    $hasCRLF = $content.Contains("`r`n")
    Write-Output "File uses CRLF: $hasCRLF"
    $sample = $content.Substring(0, [Math]::Min(200, $content.Length))
    Write-Output "First 200 chars: $sample"
}

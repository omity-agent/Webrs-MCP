param(
    [string] $SshHost = "Fro",
    [string] $Target = "x86_64-unknown-linux-gnu"
)
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$projectRoot = Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")
$BinaryPath = Join-Path $projectRoot.Path ("target\{0}\release\web" -f $Target)
& cargo zigbuild --release --target $Target --manifest-path (Join-Path $projectRoot.Path "Cargo.toml")
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}
$resolvedBinary = Resolve-Path -LiteralPath $BinaryPath
$remoteBinary = "/tmp/web.new"
$remoteDeployScript = "/tmp/web-rs-deploy.sh"
Write-Host "Uploading $($resolvedBinary.Path) to ${SshHost}:$remoteBinary"
& scp $resolvedBinary.Path "${SshHost}:$remoteBinary"
if ($LASTEXITCODE -ne 0) {
    throw "scp failed with exit code $LASTEXITCODE"
}
$remoteScript = @'
set -euo pipefail
incoming="/tmp/web.new"
deploy_script="/tmp/web-rs-deploy.sh"
install_dir="/opt/web-mcp"
install_bin="$install_dir/web"
service_file="/etc/systemd/system/web-rs.service"
new_nginx="/etc/nginx/snippets/web-rs.locations.conf"
service_user="webrs"
old_include="/etc/nginx/snippets/web-mcp.locations.conf"
new_include="/etc/nginx/snippets/web-rs.locations.conf"
trap 'rm -f "$deploy_script"' EXIT
test -s "$incoming"
chmod 0755 "$incoming"
if ! getent group "$service_user" >/dev/null; then
    groupadd --system "$service_user"
fi
if ! id -u "$service_user" >/dev/null 2>&1; then
    useradd --system --gid "$service_user" --home-dir "$install_dir" --shell /usr/sbin/nologin "$service_user"
fi
cat > "$service_file" <<'SERVICE'
[Unit]
Description=web-rs MCP Streamable HTTP service
After=network-online.target
Wants=network-online.target
[Service]
Type=simple
User=webrs
Group=webrs
WorkingDirectory=/opt/web-mcp
ExecStart=/opt/web-mcp/web mcp --transport http
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
[Install]
WantedBy=multi-user.target
SERVICE
cat > "$new_nginx" <<'NGINX'
location = /web-mcp/health {
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto https;
    proxy_connect_timeout 5s;
    proxy_send_timeout 120s;
    proxy_read_timeout 120s;
    proxy_pass http://127.0.0.1:18080/health;
}
location = /web-mcp/mcp {
    proxy_http_version 1.1;
    proxy_buffering off;
    proxy_set_header Host $host;
    proxy_set_header Origin $http_origin;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto https;
    proxy_connect_timeout 5s;
    proxy_send_timeout 300s;
    proxy_read_timeout 300s;
    proxy_pass http://127.0.0.1:18080/mcp;
}
NGINX
for site in /etc/nginx/sites-available/* /etc/nginx/sites-enabled/*; do
    [ -e "$site" ] || continue
    if [ -f "$site" ] && grep -q --fixed-strings "$old_include" "$site"; then
        sed -i "s#$old_include#$new_include#g" "$site"
    fi
done
nginx -t
systemctl stop web-rs.service 2>/dev/null || true
install -d -o root -g root -m 0755 "$install_dir"
install -o root -g root -m 0755 "$incoming" "$install_bin"
rm -f "$incoming"
systemctl daemon-reload
systemctl enable --now web-rs.service
sleep 1
systemctl is-active --quiet web-rs.service || {
    systemctl --no-pager --full status web-rs.service || true
    exit 1
}
curl -fsS http://127.0.0.1:18080/health
systemctl reload nginx
curl -kfsS --resolve the-mars.dog:9443:127.0.0.1 https://the-mars.dog:9443/web-mcp/health
printf '\nweb-rs status: '
systemctl is-active web-rs.service
printf 'web-rs enabled: '
systemctl is-enabled web-rs.service
printf 'binary: '
file "$install_bin"
'@
$remoteScript = $remoteScript -replace "`r`n", "`n"
if (-not $remoteScript.EndsWith("`n")) {
    $remoteScript += "`n"
}
$localDeployScript = Join-Path ([System.IO.Path]::GetTempPath()) ("web-rs-deploy-{0}.sh" -f [System.Guid]::NewGuid())
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($localDeployScript, $remoteScript, $utf8NoBom)
try {
    Write-Host "Uploading deployment script to ${SshHost}:$remoteDeployScript"
    & scp $localDeployScript "${SshHost}:$remoteDeployScript"
    if ($LASTEXITCODE -ne 0) {
        throw "scp deployment script failed with exit code $LASTEXITCODE"
    }
    & ssh $SshHost "bash $remoteDeployScript"
}
finally {
    Remove-Item -LiteralPath $localDeployScript -Force -ErrorAction SilentlyContinue
}
if ($LASTEXITCODE -ne 0) {
    throw "remote deployment failed with exit code $LASTEXITCODE"
}

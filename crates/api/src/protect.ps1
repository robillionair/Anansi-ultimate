$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSHOME 'Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1')
$path = $env:ANANSI_PERMISSION_PATH
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
if ($env:ANANSI_PERMISSION_DIRECTORY -eq '1') {
    $acl = New-Object Security.AccessControl.DirectorySecurity
    $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
} else {
    $acl = New-Object Security.AccessControl.FileSecurity
    $rule = New-Object Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', 'Allow')
}
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $path -AclObject $acl
$actual = Get-Acl -LiteralPath $path
if (-not $actual.AreAccessRulesProtected -or $actual.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'Private owner/DACL verification failed' }
$rules = @($actual.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
if ($rules.Count -eq 0) { throw 'Missing owner rule' }
foreach ($entry in $rules) {
    if ($entry.IdentityReference.Value -ne $sid.Value -or $entry.AccessControlType -ne 'Allow' -or ($entry.FileSystemRights -band [Security.AccessControl.FileSystemRights]::FullControl) -ne [Security.AccessControl.FileSystemRights]::FullControl) { throw 'Unexpected pairing access rule' }
}

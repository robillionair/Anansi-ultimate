$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$parentProcess = [Diagnostics.Process]::GetProcessById([int]$env:ANANSI_EXPORT_PARENT_PID)
$null = $parentProcess.Handle
$parentWatch = New-Object System.Windows.Forms.Timer
$parentWatch.Interval = 500
$parentWatch.add_Tick(({ if ($parentProcess.HasExited) { [Environment]::Exit(0) } }).GetNewClosure())
$parentWatch.Start()
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Title = 'Save complete knowledge export — choose a new filename'
$dialog.Filter = 'Portable knowledge archive (*.tar)|*.tar'
$dialog.DefaultExt = 'tar'
$dialog.FileName = 'anansi-knowledge.tar'
$dialog.OverwritePrompt = $true
try {
    if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
        [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
        [Console]::Write($dialog.FileName)
    }
} finally { $dialog.Dispose(); $parentWatch.Stop(); $parentWatch.Dispose(); $parentProcess.Dispose() }

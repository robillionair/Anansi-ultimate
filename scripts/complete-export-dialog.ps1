param([Parameter(Mandatory)][int]$DesktopProcessId,[Parameter(Mandatory)][string]$Destination)
$ErrorActionPreference='Stop'
$resolvedDestination=[IO.Path]::GetFullPath($Destination)
if ($resolvedDestination -notmatch 'anansi-export-adapter-' -or [IO.Path]::GetExtension($resolvedDestination) -ne '.tar' -or (Test-Path -LiteralPath $resolvedDestination)) { throw 'Verifier destination must be a new synthetic export archive' }
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
# Some Windows dialog providers expose native controls as UIA panes without patterns.
# Use bounded standard messages only after validating exact ownership/class/control IDs.
Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class AnansiOwnedExportDialog {
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
    [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr h);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h,StringBuilder text,int count);
    [DllImport("user32.dll")] static extern bool IsChild(IntPtr parent,IntPtr child);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll",EntryPoint="SendMessageTimeoutW")] static extern IntPtr SelectText(IntPtr h,uint message,UIntPtr parameter,IntPtr end,uint flags,uint timeout,out UIntPtr result);
    [DllImport("user32.dll",EntryPoint="SendMessageTimeoutW",CharSet=CharSet.Unicode)] static extern IntPtr SetText(IntPtr h,uint message,UIntPtr parameter,string text,uint flags,uint timeout,out UIntPtr result);
    [DllImport("user32.dll",EntryPoint="SendMessageTimeoutW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr h,uint message,UIntPtr parameter,StringBuilder text,uint flags,uint timeout,out UIntPtr result);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern bool PostMessage(IntPtr h,uint message,UIntPtr w,IntPtr l);
    static IntPtr Handle(int value) { return new IntPtr((long)(uint)value); }
    static bool Owned(IntPtr h,uint pid) { uint actual;GetWindowThreadProcessId(h,out actual);return h!=IntPtr.Zero && actual==pid; }
    static bool Class(IntPtr h,string expected) { var name=new StringBuilder(64);GetClassName(h,name,64);return name.ToString()==expected; }
    public static bool Complete(int windowValue,int editValue,int buttonValue,uint pid,string destination) {
        var window=Handle(windowValue);var edit=Handle(editValue);var button=Handle(buttonValue);
        if(!Owned(window,pid)||!Owned(edit,pid)||!Owned(button,pid)||!IsChild(window,edit)||!IsChild(window,button)||!Class(edit,"Edit")||!Class(button,"Button")||GetDlgCtrlID(edit)!=1001||GetDlgCtrlID(button)!=1)return false;
        UIntPtr result;
        if(SelectText(edit,0x00B1,UIntPtr.Zero,new IntPtr(-1),0x0002,2000,out result)==IntPtr.Zero)return false;
        if(SetText(edit,0x00C2,new UIntPtr(1),destination,0x0002,2000,out result)==IntPtr.Zero)return false;
        var selected=new StringBuilder(destination.Length+2);
        if(ReadText(edit,0x000D,new UIntPtr((uint)selected.Capacity),selected,0x0002,2000,out result)==IntPtr.Zero||selected.ToString()!=destination)return false;
        SetForegroundWindow(window);
        return PostMessage(button,0x00F5,UIntPtr.Zero,IntPtr.Zero);
    }
}
"@
for ($attempt=0;$attempt -lt 80;$attempt++) {
    $ownedProcesses=@(Get-CimInstance Win32_Process -Filter "ParentProcessId=$DesktopProcessId" | Where-Object Name -eq 'powershell.exe' | Select-Object -ExpandProperty ProcessId)
    foreach ($ownedProcess in $ownedProcesses) {
        $condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty,[int]$ownedProcess)
        $windows=[System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,$condition)
        foreach ($window in $windows) {
            if ($window.Current.Name -notlike 'Save complete knowledge export*') { continue }
            $edit=$null;$save=$null
            $controls=$window.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
            foreach ($control in $controls) {
                if ($control.Current.AutomationId -eq '1001' -and $control.Current.ClassName -eq 'Edit') {$edit=$control}
                if ($control.Current.AutomationId -eq '1' -and $control.Current.ClassName -eq 'Button') {$save=$control}
            }
            if ($null -ne $edit -and $null -ne $save -and [AnansiOwnedExportDialog]::Complete($window.Current.NativeWindowHandle,$edit.Current.NativeWindowHandle,$save.Current.NativeWindowHandle,[uint32]$ownedProcess,$resolvedDestination)) {
                Write-Output 'PASS: owned native save dialog selected a synthetic destination'
                exit 0
            }
        }
    }
    Start-Sleep -Milliseconds 250
}
throw 'Owned native export save dialog did not become available'

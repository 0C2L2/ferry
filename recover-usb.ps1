# Recovers a USB stick left with no partitions (e.g. after a failed
# partitioning run) back to a normal, usable single-partition exFAT drive.
#
# Safety: refuses to touch anything that is not a USB-bus disk, shows you the
# target and waits for explicit confirmation before writing. It does NOT run
# any destructive wipe — it only creates a partition on a disk that already
# has none, so a disk with existing partitions is rejected outright.
#
# Run from an ELEVATED PowerShell:
#   powershell -ExecutionPolicy Bypass -File .\recover-usb.ps1

$ErrorActionPreference = "Stop"

$usbDisks = Get-Disk | Where-Object { $_.BusType -eq 'USB' }

if (-not $usbDisks) {
    Write-Host "No USB disks found. Is the stick plugged in?" -ForegroundColor Yellow
    exit 1
}

Write-Host "USB disks found:" -ForegroundColor Cyan
$usbDisks | Select-Object Number, FriendlyName,
    @{N='SizeGB';E={[math]::Round($_.Size/1GB,1)}}, PartitionStyle |
    Format-Table -AutoSize

if ($usbDisks.Count -gt 1) {
    Write-Host "More than one USB disk is attached. Unplug the others and re-run," -ForegroundColor Yellow
    Write-Host "so there is no chance of picking the wrong one." -ForegroundColor Yellow
    exit 1
}

$disk = $usbDisks[0]
$existing = Get-Partition -DiskNumber $disk.Number -ErrorAction SilentlyContinue

if ($existing) {
    Write-Host "Disk $($disk.Number) already has $($existing.Count) partition(s)." -ForegroundColor Yellow
    Write-Host "This script only recovers a disk with NO partitions, so it will not" -ForegroundColor Yellow
    Write-Host "touch this one. Use Disk Management if you want to reformat it." -ForegroundColor Yellow
    exit 1
}

Write-Host ""
Write-Host "About to create one exFAT partition filling disk $($disk.Number):" -ForegroundColor Cyan
Write-Host "  $($disk.FriendlyName)  $([math]::Round($disk.Size/1GB,1)) GB  (BusType: $($disk.BusType))"
Write-Host "This disk currently has no partitions, so nothing is being erased." -ForegroundColor Green
Write-Host ""
$answer = Read-Host "Type YES to proceed"

if ($answer -ne "YES") {
    Write-Host "Cancelled, nothing changed." -ForegroundColor Yellow
    exit 0
}

if ($disk.PartitionStyle -eq 'RAW') {
    Write-Host "Initializing disk as MBR..."
    Initialize-Disk -Number $disk.Number -PartitionStyle MBR
}

Write-Host "Creating partition..."
$part = New-Partition -DiskNumber $disk.Number -UseMaximumSize -AssignDriveLetter

Write-Host "Formatting as exFAT..."
Format-Volume -Partition $part -FileSystem exFAT -NewFileSystemLabel "USB" -Confirm:$false | Out-Null

Write-Host ""
Write-Host "Done. Drive $($part.DriveLetter): is ready to use." -ForegroundColor Green
Get-Volume -DriveLetter $part.DriveLetter |
    Select-Object DriveLetter, FileSystemLabel, FileSystemType,
        @{N='SizeGB';E={[math]::Round($_.Size/1GB,1)}} | Format-Table -AutoSize

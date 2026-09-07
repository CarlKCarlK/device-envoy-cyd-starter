param(
    [string]$BusId = ''
)

$ErrorActionPreference = 'Stop'

if (-not (Get-Command usbipd -ErrorAction SilentlyContinue)) {
    Write-Error 'usbipd-win is not installed. In Windows PowerShell, run: winget install --interactive --exact dorssel.usbipd-win'
    exit 1
}

# USB-UART chips commonly fitted to classic CYD boards.
$vidPids = @(
    '1a86:7523', # CH340/CH341
    '1a86:7522', # CH340 alternate PID
    '1a86:55d3', # CH343
    '1a86:55d4', # CH9102
    '10c4:ea60', # CP2102
    '10c4:ea70', # CP2105
    '0403:6001'  # FTDI USB-UART
)

$usbList = usbipd list
$devices = @()

foreach ($line in $usbList) {
    $match = [regex]::Match(
        $line,
        '^\s*(?<busid>\d+-\d+)\s+(?<vidpid>[0-9a-fA-F]{4}:[0-9a-fA-F]{4})\s+(?<description>.+?)\s{2,}(?<state>.+)$'
    )
    if (-not $match.Success) {
        continue
    }

    $vidPid = $match.Groups['vidpid'].Value.ToLowerInvariant()
    if ($vidPids -notcontains $vidPid) {
        continue
    }

    $devices += [PSCustomObject]@{
        BusId = $match.Groups['busid'].Value
        VidPid = $vidPid
        Description = $match.Groups['description'].Value.Trim()
        State = $match.Groups['state'].Value.Trim()
    }
}

if ($BusId) {
    $devices = @($devices | Where-Object { $_.BusId -eq $BusId })
    if ($devices.Count -eq 0) {
        Write-Error "No supported CYD USB-UART adapter was found at BUSID $BusId. Run 'usbipd list' in Windows PowerShell."
        exit 1
    }
}

if ($devices.Count -eq 0) {
    Write-Error 'No CYD USB-UART adapter was found. Check the USB data cable, then run usbipd list in Windows PowerShell.'
    exit 1
}

if ($devices.Count -gt 1) {
    $attachedDevices = @($devices | Where-Object { $_.State -like 'Attached*' })
    if ($attachedDevices.Count -eq 1) {
        $devices = $attachedDevices
    } else {
        Write-Host 'More than one possible CYD USB-UART adapter was found:' -ForegroundColor Yellow
        $devices | Format-Table BusId, VidPid, Description, State
        Write-Error "Choose one with: just attach-wsl <BUSID>"
        exit 1
    }
}

$device = $devices[0]
$prefix = "[$($device.BusId) $($device.VidPid)]"

if ($device.State -like 'Attached*') {
    Write-Host "$prefix already attached to WSL" -ForegroundColor Green
    exit 0
}

if ($device.State -like '*Not shared*' -or $device.State -like '*NotShared*') {
    Write-Host "$prefix $($device.Description) is not shared with WSL." -ForegroundColor Yellow
    Write-Host 'Run this once in Windows PowerShell opened as Administrator:' -ForegroundColor Yellow
    Write-Host "  usbipd bind --busid $($device.BusId)" -ForegroundColor Cyan
    Write-Host 'Then return to WSL and run just run-esp again.' -ForegroundColor Yellow
    exit 1
}

Write-Host "$prefix attaching $($device.Description) to WSL..." -ForegroundColor Cyan
usbipd attach --wsl --busid $device.BusId
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

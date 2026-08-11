# 한글 COM 으로 목록의 HWP/HWPX 를 일괄 PDF 내보내기 — 백로그 캠페인용 오라클 캐시 생성.
#
# hwp_oracle_pdf.ps1(단건)과 달리 한 인스턴스로 목록을 순회하고(RecycleEvery 마다 재생성),
# 이미 만든 PDF 는 건너뛰어 재실행이 곧 재개다. 시작 시 HKCU CLSID 오버라이드를 지정
# 버전으로 전환하고 끝나면 머신 기본으로 원복한다(restore_com_default.ps1 과 같은 방식 —
# 키를 지우지 않고 값만 되돌린다).
#
# 전제: FilePathCheckerModule 이 HKCU\Software\HNC\HwpAutomation\Modules 에 등록돼 있어야
# 보안 대화상자가 없다. 미등록이면 Open 이 침묵-거부돼 빈 문서의 빈 PDF 가 나온다 —
# 산출물 검증은 tools/verify_oracle_pdf_cache.py 로 별도 수행한다.
#
# 사용:
#   powershell -File tools/hangul_batch_oracle_pdf.ps1 `
#     -ListPath backlog.txt -SrcRoot D:\hwpdocs_10k_share -OutRoot D:\hwpdocs_10k_share\_oracle_pdf_2022 `
#     -HwpExe 'C:\Program Files (x86)\Hnc\Office 2022\HOffice120\bin\hwp.exe'
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$ListPath,
  [Parameter(Mandatory = $true)][string]$SrcRoot,
  [Parameter(Mandatory = $true)][string]$OutRoot,
  [string]$HwpExe = 'C:\Program Files (x86)\Hnc\Office 2022\HOffice120\bin\hwp.exe',
  [int]$ExpectMajor = 12,
  [int]$RecycleEvery = 40
)
$ErrorActionPreference = 'Stop'
$CLSID = '{2291CF00-64A1-4877-A9B4-68CFE89612D6}'

function Set-ComOverride([string]$value) {
  foreach ($base in "HKCU:\Software\Classes\CLSID\$CLSID", "HKCU:\Software\Classes\Wow6432Node\CLSID\$CLSID") {
    $ls = Join-Path $base 'LocalServer32'
    New-Item -Path $ls -Force | Out-Null
    Set-ItemProperty -Path $ls -Name '(default)' -Value $value
  }
}
function Get-MachineDefault {
  $v = (Get-ItemProperty "HKLM:\SOFTWARE\Classes\WOW6432Node\CLSID\$CLSID\LocalServer32" -ErrorAction SilentlyContinue).'(default)'
  if (-not $v) { $v = (Get-ItemProperty "HKLM:\SOFTWARE\Classes\CLSID\$CLSID\LocalServer32" -ErrorAction SilentlyContinue).'(default)' }
  return $v
}
function Stop-Hwp { Get-Process Hwp -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue }
function New-Hwp {
  $h = New-Object -ComObject HWPFrame.HwpObject
  $null = $h.SetMessageBoxMode(0x00020000)
  try { $null = $h.RegisterModule("FilePathCheckDLL", "FilePathCheckerModule") } catch { }
  $major = [int](("$($h.Version)" -split '[,. ]')[0])
  if ($major -ne $ExpectMajor) { throw "Hangul major $major != expected $ExpectMajor — COM override 확인" }
  return $h
}

$files = Get-Content -LiteralPath $ListPath -Encoding UTF8 | ForEach-Object { $_.Trim().Replace('/', '\') } | Where-Object { $_ }
$machine = Get-MachineDefault
Set-ComOverride "$HwpExe -Automation"
Stop-Hwp
Start-Sleep -Milliseconds 500

$done = 0; $skipped = 0; $failed = 0
$hwp = $null
try {
  $hwp = New-Hwp
  $sinceRecycle = 0
  foreach ($src in $files) {
    if (-not (Test-Path -LiteralPath $src)) { Write-Output "MISS`t$src"; $failed++; continue }
    $rel = $src.Substring($SrcRoot.Length).TrimStart('\')
    $out = Join-Path $OutRoot ($rel + '.pdf')
    if (Test-Path -LiteralPath $out) { $skipped++; continue }
    $outDir = Split-Path -Parent $out
    if (-not (Test-Path -LiteralPath $outDir)) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }
    if ($sinceRecycle -ge $RecycleEvery) {
      try { $hwp.Quit() } catch { }
      Stop-Hwp; Start-Sleep -Milliseconds 500
      $hwp = New-Hwp; $sinceRecycle = 0
    }
    try {
      $null = $hwp.Open($src, "", "forceopen:true")
      $act = $hwp.CreateAction("FileSaveAsPdf")
      $set = $act.CreateSet()
      $null = $act.GetDefault($set)
      $set.SetItem("FileName", $out)
      $set.SetItem("Format", "PDF")
      $set.SetItem("Attributes", 0)
      $null = $act.Execute($set)
      $deadline = (Get-Date).AddSeconds(60); $last = -1
      while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 300
        if (-not (Test-Path -LiteralPath $out)) { continue }
        $size = (Get-Item -LiteralPath $out).Length
        if ($size -gt 0 -and $size -eq $last) { break }
        $last = $size
      }
      try { $null = $hwp.Clear(1) } catch { }
      $done++; $sinceRecycle++
      if (($done % 20) -eq 0) { Write-Output "# progress done=$done skipped=$skipped failed=$failed" }
    } catch {
      Write-Output "FAIL`t$src`t$($_.Exception.Message)"
      $failed++
      try { $hwp.Quit() } catch { }
      Stop-Hwp; Start-Sleep -Milliseconds 500
      $hwp = New-Hwp; $sinceRecycle = 0
    }
  }
} finally {
  try { if ($hwp) { $hwp.Quit() } } catch { }
  Stop-Hwp
  if ($machine) { Set-ComOverride $machine }
}
Write-Output "DONE done=$done skipped=$skipped failed=$failed of $($files.Count)"

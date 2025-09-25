#!/usr/bin/bash
# This script renames the file to 'SERIAL_$SERIAL_NUM.txt' and
# appends this line 'SERIAL: $SERIAL_NUM' to the top of the file.
#
# This script won't make any changes if it is not able to evaluate/find the
# $SERIAL_NUM and the filename will remain something like 'ttyUSB0-09251937.txt'
#
# As an example, this script is tailored to work with Cisco switches.
# You could start a session to make a test report by running certain
# commands and at the end, this script would rename the file to the
# switch's serial number for organizational purposes.
#
# You could imagine expanding on this script to search for a specific vendor
# or model number and make changes or apply formatting respectively.

# Get the serial number for the switch from the file
SERIAL_NUM=$(sed -n '/SN/,1p' $SERICOM_OUT_FILE | head -n 1 |
  awk -F',' '{print $3}' | awk -F': ' '{print $2}' | tr -d '\r\n')

# Only run the rest if $SERIAL_NUM is not empty
if [[ -n "$SERIAL_NUM" ]]; then
  # Craft a formatted line to append to the top of the file
  NEW_CONTENT="SERIAL: $SERIAL_NUM"

  # Append $NEW_CONTENT to the top of the file
  sed -i "1i$NEW_CONTENT" "$SERICOM_OUT_FILE"

  # Get the path of the parent directory of the
  # filename passed from sericom
  DIR=$(dirname $SERICOM_OUT_FILE | tr -d '\n')

  # Craft a new path from the original parent directory
  # and the tailored filename
  NEWPATH="$DIR/SERIAL_${SERIAL_NUM}.txt"

  # Rename/move the original file to the new filename
  mv $SERICOM_OUT_FILE $NEWPATH
fi

## Variables visualized:
#
# SERICOM_OUT_FILE - passed as an environment variable from sericom
#   /home/name/some/path/to/file/ttyUSB0-09251937.txt
# SERIAL_NUM
#   EAZ730JYRKQ
# NEW_CONTENT
#   SERIAL: EAZ730JYRKQ
# DIR
#   /home/name/some/path/to/file
# NEWPATH
#   /home/name/some/path/to/file/SERIAL_EAZ730JYRKQ.txt
#
## File contents for context:
#
# SERIAL: EAZ730JYRKQ <- this was appended to the file
# Session started at: 2025-09-25 19:37:34.135779824 UTC
#
# Switch>enable
# Switch#show inventory
# NAME: "1", DESCR: "WS-C2960X-48FPD-L"
# PID: WS-C2960X-48FPD-L , VID: V05  , SN: EAZ730JYRKQ
#
#
# NAME: "TenGigabitEthernet1/0/1", DESCR: "SFP-10GBase-SR"
# PID: SFP-10G-SR          , VID: V03  , SN: TLGCWX258MR
#
#
# NAME: "TenGigabitEthernet1/0/2", DESCR: "SFP-10GBase-SR"
# PID: SFP-10G-SR-S        , VID: V01  , SN: 5ACIR2523P3
#
#
# Switch#
# [CLOSED 2025-09-25 19:37:40.490189879 UTC] Connection closed.

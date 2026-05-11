{{/*
Expand the name of the chart.
*/}}
{{- define "hostmgr.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "hostmgr.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "hostmgr.labels" -}}
helm.sh/chart: {{ include "hostmgr.name" . }}-{{ .Chart.Version }}
{{ include "hostmgr.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "hostmgr.selectorLabels" -}}
app.kubernetes.io/name: {{ include "hostmgr.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
NATS host — use bundled sub-chart service or external
*/}}
{{- define "hostmgr.natsHost" -}}
{{- if .Values.nats.enabled }}
{{- printf "%s-nats" (include "hostmgr.fullname" .) }}
{{- else }}
{{- required "nats.externalHost is required when nats.enabled=false" .Values.nats.externalHost }}
{{- end }}
{{- end }}

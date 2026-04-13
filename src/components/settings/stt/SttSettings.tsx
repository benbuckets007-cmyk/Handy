import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";
import { Alert } from "@/components/ui/Alert";
import { Dropdown } from "@/components/ui/Dropdown";
import { Input } from "@/components/ui/Input";
import { SettingContainer } from "@/components/ui/SettingContainer";

interface SttSettingsProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const SttSettings: React.FC<SttSettingsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const provider = getSetting("stt_provider") ?? "local";
    const strategy =
      getSetting("stt_fallback_strategy") ?? "cloud_first_local_fallback";
    const model = getSetting("stt_cloud_model") ?? "MAI-Transcribe-1";
    const baseUrl = getSetting("stt_base_url") ?? "https://api.mai-ai.com/v1";
    const apiKey =
      getSetting("stt_api_keys")?.cloud ?? getSetting("stt_api_keys")?.mai ?? "";
    const connectTimeout = getSetting("stt_connect_timeout_ms") ?? 3000;
    const requestTimeout = getSetting("stt_request_timeout_ms") ?? 30000;

    const [apiKeyDraft, setApiKeyDraft] = useState(apiKey);

    useEffect(() => {
      setApiKeyDraft(apiKey);
    }, [apiKey]);

    const providerOptions = useMemo(
      () => [
        {
          value: "local",
          label: t("settings.advanced.stt.provider.options.local"),
        },
        {
          value: "cloud",
          label: t("settings.advanced.stt.provider.options.cloud"),
        },
      ],
      [t],
    );

    const strategyOptions = useMemo(
      () => [
        {
          value: "cloud_first_local_fallback",
          label: t(
            "settings.advanced.stt.fallbackStrategy.options.cloudFirstLocalFallback",
          ),
        },
        {
          value: "local_only",
          label: t("settings.advanced.stt.fallbackStrategy.options.localOnly"),
        },
        {
          value: "cloud_only",
          label: t("settings.advanced.stt.fallbackStrategy.options.cloudOnly"),
        },
      ],
      [t],
    );

    const showCloudOnlyKeyError =
      provider === "cloud" && strategy === "cloud_only" && !apiKey.trim();

    const saveApiKey = async () => {
      if (apiKeyDraft === apiKey) {
        return;
      }

      const next = {
        ...(getSetting("stt_api_keys") ?? {}),
        cloud: apiKeyDraft,
      };
      await updateSetting("stt_api_keys", next);
    };

    return (
      <div className="space-y-3">
        {showCloudOnlyKeyError && (
          <Alert variant="error" contained>
            {t("settings.advanced.stt.errors.cloudOnlyRequiresKey")}
          </Alert>
        )}

        <SettingContainer
          title={t("settings.advanced.stt.provider.title")}
          description={t("settings.advanced.stt.provider.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Dropdown
            options={providerOptions}
            selectedValue={provider}
            onSelect={(value) =>
              updateSetting("stt_provider", value as "local" | "cloud")
            }
            disabled={isUpdating("stt_provider")}
          />
        </SettingContainer>

        <SettingContainer
          title={t("settings.advanced.stt.baseUrl.title")}
          description={t("settings.advanced.stt.baseUrl.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Input
            value={baseUrl}
            onChange={(e) => updateSetting("stt_base_url", e.target.value)}
            placeholder={t("settings.advanced.stt.baseUrl.placeholder")}
            disabled={provider !== "cloud" || isUpdating("stt_base_url")}
            className="w-[220px]"
          />
        </SettingContainer>

        <SettingContainer
          title={t("settings.advanced.stt.cloudModel.title")}
          description={t("settings.advanced.stt.cloudModel.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Input
            value={model}
            onChange={(e) => updateSetting("stt_cloud_model", e.target.value)}
            placeholder={t("settings.advanced.stt.cloudModel.placeholder")}
            disabled={provider !== "cloud" || isUpdating("stt_cloud_model")}
            className="w-[220px]"
          />
        </SettingContainer>

        <SettingContainer
          title={t("settings.advanced.stt.apiKey.title")}
          description={t("settings.advanced.stt.apiKey.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Input
            value={apiKeyDraft}
            type="password"
            onChange={(e) => setApiKeyDraft(e.target.value)}
            onBlur={saveApiKey}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                void saveApiKey();
              }
            }}
            placeholder={t("settings.advanced.stt.apiKey.placeholder")}
            disabled={provider !== "cloud" || isUpdating("stt_api_keys")}
            className="w-[220px]"
          />
        </SettingContainer>

        <SettingContainer
          title={t("settings.advanced.stt.fallbackStrategy.title")}
          description={t("settings.advanced.stt.fallbackStrategy.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Dropdown
            options={strategyOptions}
            selectedValue={strategy}
            onSelect={(value) =>
              updateSetting(
                "stt_fallback_strategy",
                value as
                  | "cloud_first_local_fallback"
                  | "local_only"
                  | "cloud_only",
              )
            }
            disabled={isUpdating("stt_fallback_strategy")}
          />
        </SettingContainer>

        <SettingContainer
          title={t("settings.advanced.stt.connectTimeout.title")}
          description={t("settings.advanced.stt.connectTimeout.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Input
            type="number"
            min={1}
            value={connectTimeout}
            onChange={(e) =>
              updateSetting(
                "stt_connect_timeout_ms",
                Number(e.target.value || 0),
              )
            }
            disabled={
              provider !== "cloud" || isUpdating("stt_connect_timeout_ms")
            }
            className="w-[220px]"
          />
        </SettingContainer>

        <SettingContainer
          title={t("settings.advanced.stt.requestTimeout.title")}
          description={t("settings.advanced.stt.requestTimeout.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <Input
            type="number"
            min={1}
            value={requestTimeout}
            onChange={(e) =>
              updateSetting(
                "stt_request_timeout_ms",
                Number(e.target.value || 0),
              )
            }
            disabled={
              provider !== "cloud" || isUpdating("stt_request_timeout_ms")
            }
            className="w-[220px]"
          />
        </SettingContainer>
      </div>
    );
  },
);

SttSettings.displayName = "SttSettings";

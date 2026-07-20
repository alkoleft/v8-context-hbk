fn template_key_for_generated_self_role(
    generated_self_role: &str,
) -> Option<PlatformTypeTemplateKey> {
    let (family, variant) = match generated_self_role {
        "metadata.generated-self.catalog-object" => ("Catalog", "Object"),
        "metadata.generated-self.catalog-manager" => ("Catalog", "Manager"),
        "metadata.generated-self.document-object" => ("Document", "Object"),
        "metadata.generated-self.document-manager" => ("Document", "Manager"),
        "metadata.generated-self.information-register-record-set" => {
            ("InformationRegister", "RecordSet")
        }
        "metadata.generated-self.accumulation-register-record-set" => {
            ("AccumulationRegister", "RecordSet")
        }
        "metadata.generated-self.accounting-register-record-set" => {
            ("AccountingRegister", "RecordSet")
        }
        "metadata.generated-self.calculation-register-record-set" => {
            ("CalculationRegister", "RecordSet")
        }
        "metadata.generated-self.chart-of-characteristic-types-object" => {
            ("ChartOfCharacteristicTypes", "Object")
        }
        "metadata.generated-self.chart-of-characteristic-types-manager" => {
            ("ChartOfCharacteristicTypes", "Manager")
        }
        "metadata.generated-self.exchange-plan-object" => ("ExchangePlan", "Object"),
        "metadata.generated-self.exchange-plan-manager" => ("ExchangePlan", "Manager"),
        "metadata.generated-self.business-process-object" => ("BusinessProcess", "Object"),
        "metadata.generated-self.business-process-manager" => ("BusinessProcess", "Manager"),
        "metadata.generated-self.task-object" => ("Task", "Object"),
        "metadata.generated-self.task-manager" => ("Task", "Manager"),
        "metadata.generated-self.chart-of-accounts-object" => ("ChartOfAccounts", "Object"),
        "metadata.generated-self.chart-of-accounts-manager" => {
            ("ChartOfAccounts", "Manager")
        }
        "metadata.generated-self.chart-of-calculation-types-object" => {
            ("ChartOfCalculationTypes", "Object")
        }
        "metadata.generated-self.chart-of-calculation-types-manager" => {
            ("ChartOfCalculationTypes", "Manager")
        }
        _ => return None,
    };
    Some(PlatformTypeTemplateKey::new(family, variant))
}

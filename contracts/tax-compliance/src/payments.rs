use crate::{tax_engine, Balance, PaymentReceipt, TaxRecord, Timestamp};

pub(crate) fn apply_payment(
    mut record: TaxRecord,
    amount: Balance,
    payment_reference: [u8; 32],
    now: Timestamp,
) -> (TaxRecord, PaymentReceipt) {
    record.paid_amount = record.paid_amount.saturating_add(amount);
    record.last_payment_at = now;
    record.payment_reference = payment_reference;
    record.status = tax_engine::resolve_status(record, now);
    let receipt = PaymentReceipt {
        property_id: record.property_id,
        jurisdiction_code: record.jurisdiction_code,
        reporting_period: record.reporting_period,
        payment_reference,
        amount_paid: amount,
        outstanding_balance: outstanding_tax(&record),
        settled_at: now,
    };
    (record, receipt)
}

pub(crate) fn outstanding_tax(record: &TaxRecord) -> Balance {
    record.tax_due.saturating_sub(record.paid_amount)
}

use crate::gen::kin::agora::common::v3 as model_pb_v3;
use prost::Message;
use sha2::{Digest, Sha224};

/// Represents a line item in an invoice.
#[derive(Debug, Clone)]
pub struct LineItem {
    pub title: String,
    pub amount: i64, // The amount in quarks.
    pub description: Option<String>,
    pub sku: Option<Vec<u8>>, // The app SKU related to this line item, if applicable.
}

impl LineItem {
    pub fn new(title: String, amount: i64) -> LineItem {
        LineItem {
            title,
            amount,
            description: None,
            sku: None,
        }
    }

    pub fn from_proto(line_item: model_pb_v3::invoice::LineItem) -> LineItem {
        let mut item = LineItem::new(line_item.title, line_item.amount);

        item.description = Some(line_item.description);
        item.sku = Some(line_item.sku);

        item
    }

    pub fn to_proto(&self) -> model_pb_v3::invoice::LineItem {
        let description = self.description.clone().unwrap_or_default();
        let sku = self.sku.clone().unwrap_or_default();

        model_pb_v3::invoice::LineItem {
            title: self.title.clone(),
            amount: self.amount,
            description,
            sku,
        }
    }
}

/// Represents a transaction invoice for a single payment.
#[derive(Debug, Clone)]
pub struct Invoice {
    pub items: Vec<LineItem>,
}

impl Invoice {
    pub fn new(title: &str, amount: i64, description: Option<&str>, sku: Option<&[u8]>) -> Invoice {
        let mut item = LineItem::new(title.to_string(), amount);

        item.description = description.map(|d| d.to_string());
        item.sku = sku.map(|s| s.to_vec());

        Invoice { items: vec![item] }
    }

    pub fn from_items(items: Vec<LineItem>) -> Invoice {
        Invoice { items }
    }

    pub fn from_proto(invoice: model_pb_v3::Invoice) -> Invoice {
        let mut items = Vec::new();
        for item in invoice.items {
            items.push(LineItem::from_proto(item));
        }

        Invoice { items }
    }

    pub fn to_proto(&self) -> model_pb_v3::Invoice {
        let mut items = Vec::new();
        for item in &self.items {
            items.push(item.to_proto());
        }

        model_pb_v3::Invoice { items }
    }
}

/// Represents a list of invoices associated with a transaction.
#[derive(Debug, Clone)]
pub struct InvoiceList {
    pub invoices: Vec<Invoice>,
}

impl InvoiceList {
    pub fn new(invoices: &[Invoice]) -> InvoiceList {
        InvoiceList {
            invoices: invoices.to_vec(),
        }
    }

    pub fn from_proto(proto: model_pb_v3::InvoiceList) -> InvoiceList {
        let invoices = proto
            .invoices
            .into_iter()
            .map(Invoice::from_proto)
            .collect();

        InvoiceList { invoices }
    }

    pub fn to_proto(&self) -> model_pb_v3::InvoiceList {
        let invoices = self.invoices.iter().map(|i| i.to_proto()).collect();

        model_pb_v3::InvoiceList { invoices }
    }

    pub fn get_sha244_hash(&self) -> Vec<u8> {
        let serialized = self.to_proto().encode_to_vec();
        let mut hasher = Sha224::new();
        hasher.update(serialized);
        hasher.finalize().to_vec()
    }
}
